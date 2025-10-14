use std::path::Path;
use std::sync::LazyLock;

use async_std::sync::RwLock;

use anyhow_ext::Context;
use anyhow_ext::Result;
use serde::Deserialize;
use std::fmt::Debug;
use tracing::Level;

use crate::cli::Cli;

pub static CFG: LazyLock<RwLock<Config>> = LazyLock::new(|| RwLock::new(Config::default()));

pub async fn load_config_from_cli(cli: &Cli) -> Result<()> {
	let config = Config {
		port: cli.port,
		log_level: match cli.verbose {
			0 => LogLevel(Level::INFO),
			1 => LogLevel(Level::DEBUG),
			2.. => LogLevel(Level::TRACE),
		},
		db_url: cli.db_url.to_owned(),
	};
	let mut lock = CFG.write().await;
	*lock = config;
	Ok(())
}

pub async fn load_config<P>(config_path: P) -> Result<()>
where
	P: AsRef<Path> + Debug,
{
	let data = std::fs::read_to_string(&config_path).context(format!(
		"failed to read config file data, path={:?}",
		config_path.as_ref()
	))?;
	let config: Config = toml::from_str(data.as_str()).context(format!("{:?}", config_path))?;
	let mut lock = CFG.write().await;
	*lock = config;
	Ok(())
}

#[derive(Deserialize, Default)]
pub struct Config {
	pub port: u16,
	pub log_level: LogLevel,
	pub db_url: String,
}

pub struct LogLevel(pub tracing::Level);
impl<'de> Deserialize<'de> for LogLevel {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?.to_lowercase();
		match s.as_str() {
			"trace" => return Ok(LogLevel(Level::TRACE)),
			"debug" => return Ok(LogLevel(Level::DEBUG)),
			"info" => return Ok(LogLevel(Level::INFO)),
			"error" => return Ok(LogLevel(Level::ERROR)),
			other => {
				return Err(serde::de::Error::custom(format!(
					"cannot convert {other} to log level"
				)))
			}
		}
	}
}
impl Default for LogLevel {
	fn default() -> Self {
		LogLevel(Level::DEBUG)
	}
}

pub async fn cfg() -> async_std::sync::RwLockReadGuard<'static, Config> {
	return CFG.read().await;
}

/// Safely get a copy of the current config to avoid deadlocks
pub async fn get_config() -> Config {
	let config_guard = CFG.read().await;
	Config {
		port: config_guard.port,
		log_level: LogLevel(config_guard.log_level.0.clone()),
		db_url: config_guard.db_url.clone(),
	}
}

/// Safely get just the log level to avoid deadlocks
pub async fn get_log_level() -> tracing::Level {
	let config_guard = CFG.read().await;
	config_guard.log_level.0.clone()
}

/// Safely get just the database URL to avoid deadlocks
pub async fn get_db_url() -> String {
	let config_guard = CFG.read().await;
	config_guard.db_url.clone()
}
