use std::path::Path;
use std::sync::LazyLock;

use async_std::sync::RwLock;
use clap::Parser;

use anyhow_ext::Context;
use anyhow_ext::Result;
use derive_builder::Builder;
use serde::Deserialize;
use std::fmt::Debug;

pub static CONFIG: LazyLock<RwLock<Config>> = LazyLock::new(|| RwLock::new(Config::default()));

pub async fn load_config_from_cli(cli: Config) -> Result<()> {
	let mut lock = CONFIG.write().await;
	*lock = cli;
	Ok(())
}

pub async fn load_config_from_path(config_path: Option<&str>) -> Result<()> {
	let config_path = match config_path {
		Some(path) => path,
		None => {
			tracing::info!("No config file specified, using CLI parameters and defaults");
			return Ok(());
		}
	};

	if !Path::new(config_path).exists() {
		tracing::warn!(
			"Config file does not exist: {:?}, using CLI parameters and defaults",
			config_path
		);
		return Ok(());
	}

	let data = std::fs::read_to_string(config_path).context(format!(
		"failed to read config file data, path={:?}",
		config_path
	))?;
	let file_config: Config = toml::from_str(&data).context(format!("{:?}", config_path))?;

	let mut lock = CONFIG.write().await;
	// Merge: file config overrides CLI config (only for non-None values)
	if file_config.bind != default_addr() {
		lock.bind = file_config.bind;
	}
	if file_config.log_directive != default_log_directive() {
		lock.log_directive = file_config.log_directive;
	}
	if file_config.db_url.is_some() {
		lock.db_url = file_config.db_url;
	}
	Ok(())
}

#[derive(Deserialize, Default, Builder, Debug, Clone, Parser)]
#[builder(setter(into))]
#[command(version, about, long_about = None)]
pub struct Config {
	/// Server address (e.g., "0.0.0.0:8888")
	#[arg(short, long, default_value = "0.0.0.0:8888")]
	#[serde(default = "default_addr")]
	pub bind: String,

	/// Log directive (e.g., "info,tide=warn", "debug", "sqlx=error")
	#[arg(short, long, default_value = "info,tide=warn")]
	#[serde(default = "default_log_directive")]
	pub log_directive: String,

	/// Database URL (optional)
	#[arg(short, long)]
	#[serde(default)]
	pub db_url: Option<String>,

	/// Sets a custom config file (optional)
	#[arg(short, long, value_name = "FILE")]
	#[serde(default)]
	pub config_file: Option<String>,
}

fn default_addr() -> String {
	"0.0.0.0:8888".to_string()
}

fn default_log_directive() -> String {
	"info,tide=warn".to_string()
}

/// Get a read lock guard for zero-copy access to the global config.
///
/// # Returns
///
/// A `RwLockReadGuard` that allows direct field access without cloning.
/// The guard is automatically released when it goes out of scope.
///
/// # Examples
///
/// ## Access fields without cloning (zero-copy)
/// ```no_run
/// # use rust_tide_template::config;
/// let cfg = config::cfg().await;
/// println!("Binding to: {}", cfg.bind);
/// println!("Log directive: {}", cfg.log_directive);
/// // Guard is released here when cfg goes out of scope
/// ```
///
/// ## Clone fields if you need them beyond the guard's lifetime
/// ```no_run
/// # use rust_tide_template::config;
/// let cfg = config::cfg().await;
/// let bind = cfg.bind.clone();
/// let directive = cfg.log_directive.clone();
/// // Use cloned values after guard is released
/// ```
///
/// ## Clone entire config (convenient, has small overhead)
/// ```no_run
/// # use rust_tide_template::config;
/// let config = config::cfg().await.clone();
/// // You own the cloned config, no lock guard to worry about
/// ```
///
/// # Performance Notes
///
/// - **Zero-copy**: Access fields directly through the guard (highest performance)
/// - **Clone one field**: When you only need one specific value
/// - **Clone entire struct**: When you need multiple fields and want ownership (minimal overhead for small structs)
///
/// # ⚠️ Deadlock Warning
///
/// **NEVER** call `cfg()` again while holding a guard**:
///
/// ```no_run
/// // ❌ DEADLOCK! Don't do this!
/// let guard1 = config::cfg().await;     // Holds read lock
/// let guard2 = config::cfg().await;     // Tries to acquire read lock again → DEADLOCK
/// ```
///
/// The `async_std::sync::RwLock` is not reentrant, so you cannot acquire the lock
/// multiple times in the same call stack. Always release the guard (by letting it
/// go out of scope) before calling `cfg()` again.
pub async fn cfg() -> async_std::sync::RwLockReadGuard<'static, Config> {
	return CONFIG.read().await;
}

/// Update the global config with a new Config instance.
///
/// This completely replaces the existing config with the provided one.
/// Use this when you need to update multiple fields at once atomically.
///
/// # Example
/// ```no_run
/// # use rust_tide_template::config::Config;
/// # use rust_tide_template::config;
/// # async fn example() {
/// let new_config = Config {
///     bind: "127.0.0.1:8080".to_string(),
///     ..Default::default()
/// };
/// config::set_cfg(new_config).await;
/// # }
/// ```
pub async fn set_cfg(config: Config) {
	let mut lock = CONFIG.write().await;
	*lock = config;
}

