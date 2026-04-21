use std::path::Path;
use std::sync::LazyLock;

use async_std::sync::RwLock;
use clap::Parser;

use anyhow_ext::Context;
use anyhow_ext::Result;
use serde::Deserialize;
use std::fmt::Debug;

pub static CONFIG: LazyLock<RwLock<Config>> = LazyLock::new(|| RwLock::new(Config::default()));

#[derive(Deserialize, Default, Debug, Clone, Parser)]
#[serde(default)]
pub struct RawConfig {
	/// Server address (e.g., "0.0.0.0:8888")
	#[arg(
		short,
		long,
		env = "APP_BIND",
		help = "Server address [default: 0.0.0.0:8888]"
	)]
	pub bind: Option<String>,

	/// Log directive (e.g., "info,tide=warn", "debug", "sqlx=error")
	#[arg(
		short,
		long,
		env = "APP_LOG_DIRECTIVE",
		help = "Log directive [default: info,tide=warn]"
	)]
	pub log_directive: Option<String>,

	/// Database URL (optional)
	#[arg(short, long, env = "APP_DB_URL", help = "Database URL (optional)")]
	pub db_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
	pub bind: String,
	pub log_directive: String,
	pub db_url: Option<String>,
	pub config_file: Option<String>,
}

fn default_addr() -> String {
	"0.0.0.0:8888".to_string()
}

fn default_log_directive() -> String {
	"info,tide=warn".to_string()
}

/// Merge config sources with priority: CLI > config file > env var > default.
///
/// Since clap already handles CLI > env var internally (CLI arg wins over env),
/// `cli` contains the result of that merge. We then layer the config file
/// between "explicitly set by CLI/env" and "default".
pub fn merge(cli: RawConfig, file: RawConfig) -> Config {
	Config {
		bind: cli.bind.or(file.bind).unwrap_or_else(default_addr),
		log_directive: cli
			.log_directive
			.or(file.log_directive)
			.unwrap_or_else(default_log_directive),
		db_url: cli.db_url.or(file.db_url),
		config_file: None,
	}
}

pub fn load_config_file(path: &str) -> Result<RawConfig> {
	if !Path::new(path).exists() {
		tracing::warn!(
			"Config file does not exist: {:?}, using CLI parameters and defaults",
			path
		);
		return Ok(RawConfig::default());
	}

	let data = std::fs::read_to_string(path)
		.dot()
		.context(format!("failed to read config file, path={:?}", path))?;
	let file_config: RawConfig = toml::from_str(&data)
		.dot()
		.context(format!("failed to parse config file, path={:?}", path))?;
	Ok(file_config)
}

pub async fn load_config(cli: RawConfig, config_file_path: Option<&str>) -> Result<()> {
	let file_config = match config_file_path {
		Some(path) => load_config_file(path)?,
		None => {
			tracing::info!("No config file specified, using CLI parameters and defaults");
			RawConfig::default()
		}
	};

	let mut config = merge(cli, file_config);
	config.config_file = config_file_path.map(|s| s.to_string());

	let mut lock = CONFIG.write().await;
	*lock = config;
	Ok(())
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_merge_all_defaults() {
		let cli = RawConfig::default();
		let file = RawConfig::default();
		let config = merge(cli, file);

		assert_eq!(config.bind, "0.0.0.0:8888");
		assert_eq!(config.log_directive, "info,tide=warn");
		assert_eq!(config.db_url, None);
	}

	#[test]
	fn test_merge_cli_overrides_file() {
		let cli = RawConfig {
			bind: Some("127.0.0.1:3000".to_string()),
			log_directive: Some("debug".to_string()),
			db_url: Some("sqlite:cli.db".to_string()),
		};
		let file = RawConfig {
			bind: Some("0.0.0.0:9999".to_string()),
			log_directive: Some("warn".to_string()),
			db_url: Some("sqlite:file.db".to_string()),
		};
		let config = merge(cli, file);

		assert_eq!(config.bind, "127.0.0.1:3000");
		assert_eq!(config.log_directive, "debug");
		assert_eq!(config.db_url, Some("sqlite:cli.db".to_string()));
	}

	#[test]
	fn test_merge_file_fills_defaults() {
		let cli = RawConfig::default();
		let file = RawConfig {
			bind: Some("0.0.0.0:9999".to_string()),
			log_directive: Some("warn".to_string()),
			db_url: Some("sqlite:file.db".to_string()),
		};
		let config = merge(cli, file);

		assert_eq!(config.bind, "0.0.0.0:9999");
		assert_eq!(config.log_directive, "warn");
		assert_eq!(config.db_url, Some("sqlite:file.db".to_string()));
	}

	#[test]
	fn test_merge_cli_partial_overrides() {
		let cli = RawConfig {
			bind: Some("127.0.0.1:3000".to_string()),
			..Default::default()
		};
		let file = RawConfig {
			bind: Some("0.0.0.0:9999".to_string()),
			log_directive: Some("warn".to_string()),
			db_url: Some("sqlite:file.db".to_string()),
		};
		let config = merge(cli, file);

		assert_eq!(config.bind, "127.0.0.1:3000");
		assert_eq!(config.log_directive, "warn");
		assert_eq!(config.db_url, Some("sqlite:file.db".to_string()));
	}

	#[test]
	fn test_merge_file_partial_fills() {
		let cli = RawConfig {
			log_directive: Some("debug".to_string()),
			..Default::default()
		};
		let file = RawConfig {
			bind: Some("0.0.0.0:9999".to_string()),
			..Default::default()
		};
		let config = merge(cli, file);

		assert_eq!(config.bind, "0.0.0.0:9999");
		assert_eq!(config.log_directive, "debug");
		assert_eq!(config.db_url, None);
	}

	#[test]
	fn test_load_config_file_valid() {
		let dir = std::env::temp_dir().join("rust_tide_template_test_config");
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("test_config.toml");
		std::fs::write(
			&path,
			r#"bind = "0.0.0.0:9999"
log_directive = "warn"
db_url = "sqlite:test.db"
"#,
		)
		.unwrap();

		let raw = load_config_file(path.to_str().unwrap()).unwrap();
		assert_eq!(raw.bind, Some("0.0.0.0:9999".to_string()));
		assert_eq!(raw.log_directive, Some("warn".to_string()));
		assert_eq!(raw.db_url, Some("sqlite:test.db".to_string()));

		std::fs::remove_dir_all(&dir).ok();
	}

	#[test]
	fn test_load_config_file_partial() {
		let dir = std::env::temp_dir().join("rust_tide_template_test_config_partial");
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("partial.toml");
		std::fs::write(
			&path,
			r#"bind = "0.0.0.0:7777"
"#,
		)
		.unwrap();

		let raw = load_config_file(path.to_str().unwrap()).unwrap();
		assert_eq!(raw.bind, Some("0.0.0.0:7777".to_string()));
		assert_eq!(raw.log_directive, None);
		assert_eq!(raw.db_url, None);

		std::fs::remove_dir_all(&dir).ok();
	}

	#[test]
	fn test_load_config_file_not_found() {
		let raw = load_config_file("/nonexistent/path/config.toml").unwrap();
		assert_eq!(raw.bind, None);
	}

	#[test]
	fn test_load_config_file_invalid_toml() {
		let dir = std::env::temp_dir().join("rust_tide_template_test_config_invalid");
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("invalid.toml");
		std::fs::write(&path, r#"this is not valid toml [[[["#).unwrap();

		let result = load_config_file(path.to_str().unwrap());
		assert!(result.is_err());

		std::fs::remove_dir_all(&dir).ok();
	}

	#[test]
	fn test_config_default() {
		let config = Config::default();
		assert_eq!(config.bind, "");
		assert_eq!(config.log_directive, "");
		assert_eq!(config.db_url, None);
		assert_eq!(config.config_file, None);
	}
}
