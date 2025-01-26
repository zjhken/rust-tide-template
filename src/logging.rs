use std::time::SystemTime;
use std::{fmt::Debug, sync::OnceLock};

use async_std::sync::Mutex;

use anyhow_ext::Context;
use anyhow_ext::Result;
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::fmt::format;

use crate::auth;
use crate::common::gen_n_random_str;
use crate::config::LogLevel;
use tracing_subscriber::{
	filter,
	fmt::{self, layer},
	prelude::*,
	reload::{self, Handle},
	Registry,
};

use std::time::{Duration, Instant};

use surf::StatusCode;
use tide::Response;
use tide::{Middleware, Next, Request};
use tracing::{debug, error, error_span, info, info_span, warn, warn_span, Instrument};

// TODO: use fastrace https://github.com/fast/fastrace/blob/main/fastrace/examples/asynchronous.rs#L13

async_std::task_local! {
	static REQ_ID: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

pub fn setup_logger(log_level: &Level) -> Result<()> {
	fern::Dispatch::new()
		.format(|out, message, record| {
			let mut req_id = String::new();
			REQ_ID.with(|id| req_id = id.borrow().to_string());
			if req_id.is_empty() {
				out.finish(format_args!(
					"{timestamp}|{level}|{target}|{message}",
					timestamp = humantime::format_rfc3339_millis(SystemTime::now()),
					level = record.level(),
					target = record.target(),
				));
			} else if record.level() == log::Level::Error {
				out.finish(format_args!(
					"{timestamp}|{level}|{target}|{req_id}|{file}@{line}|{message}",
					timestamp = humantime::format_rfc3339_millis(SystemTime::now()),
					level = record.level(),
					target = record.target(),
					file = record.file().unwrap_or("unknown_file"),
					line = record.line().unwrap_or(0),
				));
			} else {
				out.finish(format_args!(
					"{timestamp}|{level}|{req_id}|{message}",
					timestamp = humantime::format_rfc3339_millis(SystemTime::now()),
					level = record.level(),
				));
			}
		})
		.level(match log_level {
			&tracing::Level::ERROR => log::LevelFilter::Error,
			&tracing::Level::WARN => log::LevelFilter::Warn,
			&tracing::Level::INFO => log::LevelFilter::Info,
			&tracing::Level::DEBUG => log::LevelFilter::Debug,
			&tracing::Level::TRACE => log::LevelFilter::Trace,
		})
		.filter(|metadata| !metadata.target().starts_with("tide::log::middleware"))
		.chain(std::io::stderr())
		.apply()?;
	Ok(())
}

static RELOAD_HANDLE: OnceLock<Mutex<Handle<LevelFilter, Registry>>> = OnceLock::new();
pub fn setup_logger_with_tracing_subscriber(log_level: &Level) -> Result<()> {
	let target_filter_layer = filter::Targets::new()
		.with_targets(vec![
			("async_io::driver", Level::TRACE),
			("tide", Level::DEBUG),
			("excel_to_web", Level::DEBUG),
		])
		.with_target("tide::log::middleware", LevelFilter::OFF)
		.with_target("polling::epoll", LevelFilter::OFF);

	let level: LevelFilter = log_level.to_owned().into();
	let (level_filter, reload_handle) = reload::Layer::new(level);
	RELOAD_HANDLE.get_or_init(|| Mutex::new(reload_handle));

	tracing_subscriber::registry()
		.with(
			tracing_subscriber::fmt::layer()
				.compact()
				.with_writer(std::io::stderr)
				.with_filter(level_filter), // .with_filter(filter),
		)
		// .with(tracing_subscriber::fmt::layer().compact()) // this will add one more output, by default is stdout
		// .with(log_level_filter_layer)
		.with(target_filter_layer)
		.init();

	Ok(())
}

pub async fn reload_log_level(log_level: impl Into<LevelFilter>) -> Result<()> {
	RELOAD_HANDLE
		.get()
		.unwrap()
		.lock()
		.await
		.reload(log_level)
		.dot()?;
	Ok(())
}

pub fn init_logger_old(log_level: &LogLevel) -> Result<()> {
	let subscriber = tracing_subscriber::FmtSubscriber::builder()
		.with_writer(std::io::stderr)
		.with_max_level(log_level.0)
		.finish();
	tracing::subscriber::set_global_default(subscriber)?;

	// let filter = convert_to_level_filter_level(log_level);
	// let (filter, reload_handle) =
	//     reload::Layer::<tracing::level_filters::LevelFilter, Registry>::new(filter);

	// let registry = Registry::default();
	// let layer = layer().with_writer(std::io::stderr);
	// registry.with(layer).with(filter).init();

	Ok(())
}

#[derive(Debug, Default, Clone)]
pub struct AccessLogMiddleware;
impl AccessLogMiddleware {}
#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> tide::Middleware<State> for AccessLogMiddleware {
	async fn handle(&self, req: tide::Request<State>, next: tide::Next<'_, State>) -> tide::Result {
		let path = req.url().path().to_owned();
		let method = req.method();
		let ip = req.peer_addr().unwrap_or("-").to_string();
		let username = auth::read_cred_from_basic_auth(&req)
			.map(|cred| cred.username)
			.unwrap_or("-".to_owned());
		REQ_ID.with(|id| {
			let mut id = id.borrow_mut();
			id.clear();
			id.push_str(gen_n_random_str(6).as_str());
		});
		let agent = req.header("user-agent").map(|a| a.as_str()).unwrap_or("-");
		let agent = agent
			.split_once(' ')
			.map(|(a, _)| a)
			.unwrap_or("-")
			.to_string();
		let req_body_size = req.len().unwrap_or(0);

		let start = Instant::now();

		let response = next.run(req).await;

		let size = match method {
			tide::http::Method::Post => req_body_size,
			_ => response.len().unwrap_or(0),
		};
		let duration = start.elapsed();
		let status = response.status();

		let access_log_msg =
			format!("{ip}|{agent}|{username}|{method}|{status}|{duration:?}|{size}B|{path}",);
		info!("{access_log_msg}");

		return Ok(response);
	}
}
