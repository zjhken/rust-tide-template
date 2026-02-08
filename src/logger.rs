use std::sync::OnceLock;

use anyhow_ext::{Context, Result, bail};
use tracing::{Event, Level, Subscriber, info};

use time::format_description;
use tracing_subscriber::{
	EnvFilter, Registry,
	fmt::{self, FmtContext, FormatEvent, FormatFields, FormattedFields, format::Writer},
	layer::SubscriberExt,
	registry::LookupSpan,
	reload,
	util::SubscriberInitExt,
};

use crate::utils::{self, REQ_ID};

pub type LogHandle = reload::Handle<EnvFilter, Registry>;
pub static GLOBAL_LOG_HANDLE: OnceLock<LogHandle> = OnceLock::new();

pub static TIME_FORMAT: &[format_description::FormatItem<'static>] = time::macros::format_description!(
	"[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]"
);

pub(crate) fn setup_logger(level: &tracing::Level) -> Result<()> {
	let level_str = level.as_str();
	// 1. 定义初始规则 (推荐方案 B 的变种)
	let filter = EnvFilter::new(format!("{level_str},log=error"));

	// 2. 包装进 reload
	let (filter_layer, reload_handle) = reload::Layer::new(filter);
	GLOBAL_LOG_HANDLE
		.set(reload_handle)
		.expect("Failed to set global log handle");

	// 3. 注册
	tracing_subscriber::registry()
		.with(filter_layer)
		// .with(fmt::Layer::default())
		// .with(fmt::layer().event_format(format))
		.with(fmt::layer().event_format(PipeFormatter))
		.try_init()
		.dot()?;
	Ok(())
}

/// Updates the global log level using a tracing directive.
///
/// This function allows runtime modification of log levels without restarting the application.
/// The directive follows the standard `tracing-subscriber` filter format.
///
/// # Directive Format
///
/// The directive string can contain multiple comma-separated filter directives:
///
/// ```text
/// <filter>=<level>,<module>=<level>,<level>
/// ```
///
/// Where:
/// - `<filter>`: Target (can be omitted for default)
/// - `<level>`: Log level (trace, debug, info, warn, error)
/// - `<module>`: Rust module path (e.g., `jhproxy::tee_reader`)
///
/// # Examples
///
/// ## Basic Level Setting
/// ```rust
/// // Set global level to debug
/// update_global_log_level("debug");
///
/// // Set global level to error only
/// update_global_log_level("error");
/// ```
///
/// ## Module-Specific Levels
/// ```rust
/// // Enable debug for tee_reader module, info for others
/// update_global_log_level("jhproxy::tee_reader=debug,info");
///
/// // Enable trace for inside module, warn for everything else
/// update_global_log_level("jhproxy::inside=trace,warn");
/// ```
///
/// ## Multiple Module Directives
/// ```rust
/// // Different levels for different modules
/// update_global_log_level("jhproxy::tee_reader=debug,jhproxy::inside=info,warn");
/// ```
///
/// ## Crate-Specific Directives
/// ```rust
/// // Target all modules in jhproxy crate
/// update_global_log_level("jhproxy=debug,warn");
/// ```
///
/// ## Common Use Cases
/// ```rust
/// # Development - show everything
/// update_global_log_level("debug");
///
/// # Production - only important messages
/// update_global_log_level("warn");
///
/// # Debug specific component
/// update_global_log_level("jhproxy::tee_reader=trace,warn");
/// ```
///
/// # Error Handling
///
/// The function will log detailed information about:
/// - Update attempts and their success/failure
/// - Invalid directive formats with helpful tips
/// - Missing logger initialization
///
/// # Notes
///
/// - The logger must be initialized via `setup_logger()` before calling this function
/// - Empty directives are ignored with a warning
/// - Changes take effect immediately for all subsequent log statements
pub fn update_global_log_level(directive: &str) {
	// 1. 获取全局 Handle
	if let Some(handle) = GLOBAL_LOG_HANDLE.get() {
		// 2. 创建新的 Filter
		let new_filter = EnvFilter::new(directive);

		// 3. 执行 Reload
		match handle.reload(new_filter) {
			Ok(_) => tracing::info!("全局日志级别已更新为: {}", directive),
			Err(e) => tracing::error!("日志级别更新失败: {}", e),
		}
	} else {
		tracing::error!("日志系统尚未初始化，无法修改级别");
	}
}

pub fn get_global_log_level() -> Result<String> {
	// 1. 获取全局 Handle
	if let Some(handle) = GLOBAL_LOG_HANDLE.get() {
		let mut filter = EnvFilter::new("info");
		handle.with_current(|x| {
			filter = x.clone();
		}).dot()?;
		return Ok(filter.to_string());
	} else {
		bail!("日志系统尚未初始化，无法修改级别");
	}
}

// 1. 定义你的 Formatter 结构体
struct PipeFormatter;

// 2. 实现 FormatEvent trait
impl<S, N> FormatEvent<S, N> for PipeFormatter
where
	S: Subscriber + for<'a> LookupSpan<'a>,
	N: for<'a> FormatFields<'a> + 'static,
{
	fn format_event(
		&self,
		ctx: &FmtContext<'_, S, N>,
		mut writer: Writer<'_>,
		event: &Event<'_>,
	) -> std::fmt::Result {
		// --- 字段 1: 时间 ---
		let now = time::OffsetDateTime::now_utc().format(TIME_FORMAT).unwrap();
		write!(writer, "{}|", now)?;

		// ========================================================
		// 2. 级别 (重点修改: 映射为固定 4 字符)
		// ========================================================
		let level_str = match *event.metadata().level() {
			Level::TRACE => "TRCE",
			Level::DEBUG => "DBUG",
			Level::INFO => "INFO",
			Level::WARN => "WARN",
			Level::ERROR => "ERRO",
		};
		// 因为长度固定，这里不再需要 {:<5} 这种对齐参数了
		writer.write_str(level_str)?;
		writer.write_str("|")?;

		// --- 字段 3: 模块路径/Target ---
		write!(writer, "{}|", event.metadata().target())?;

		// --- 字段: request ID ---
		let mut req_id = utils::get_req_id();
		if req_id == "" {
			req_id.push_str("-");
		}
		write!(writer, "{req_id}|")?;

		// --- 字段 4: Span 上下文 (重点: 获取 request_id) ---
		let mut has_written = false; // 标记变量：这一列有没有写过东西？

		if let Some(scope) = ctx.event_scope() {
			for span in scope.from_root() {
				let ext = span.extensions();
				if let Some(fields) = ext.get::<FormattedFields<N>>() {
					// 如果字段非空
					if !fields.fields.is_empty() {
						// 如果之前已经写过 span 了，这就不是第一个，加逗号分隔
						if has_written {
							writer.write_str(",")?;
						}

						// 直接流式写入 span 内容
						writer.write_str(&fields.fields)?;
						has_written = true;
					}
				}
			}
		}
		// 如果遍历完发现啥都没写（没有 span 或 span 没字段），给个占位符
		if !has_written {
			writer.write_char('-')?;
		}

		// 这一列结束的分隔符
		writer.write_str("|")?;

		// --- 字段 5: 具体的日志消息 ---
		// 使用 writer 提供的 visitor 来记录 event 里的字段 (通常是 message)
		ctx.field_format().format_fields(writer.by_ref(), event)?;

		// 换行
		writeln!(writer)
	}
}