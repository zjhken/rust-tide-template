use std::path::Path;
use std::sync::LazyLock;

use async_std::sync::RwLock;

use anyhow_ext::Context;
use anyhow_ext::Result;
use derive_builder::Builder;
use serde::Deserialize;
use std::fmt::Debug;

use crate::cli::Cli;

pub static CFG: LazyLock<RwLock<Config>> = LazyLock::new(|| RwLock::new(Config::default()));

pub async fn load_config_from_cli(cli: &Cli) -> Result<()> {
	let directive = match cli.verbose {
		0 => "info,tide=warn",
		1 => "debug,tide=warn",
		2.. => "debug,tide=warn", // trace causes async-std panic, use debug instead
	};
	let config = Config {
		bind: cli.bind.clone(),
		log_directive: directive.to_string(),
		db_url: cli.db_url.clone(),
	};
	let mut lock = CFG.write().await;
	*lock = config;
	Ok(())
}

pub async fn load_config<P>(config_path: Option<P>) -> Result<()>
where
	P: AsRef<Path> + Debug,
{
	let config_path = match config_path {
		Some(path) => path,
		None => {
			tracing::info!("No config file specified, using CLI parameters and defaults");
			return Ok(());
		}
	};

	if !config_path.as_ref().exists() {
		tracing::warn!(
			"Config file does not exist: {:?}, using CLI parameters and defaults",
			config_path.as_ref()
		);
		return Ok(());
	}

	let data = std::fs::read_to_string(&config_path).context(format!(
		"failed to read config file data, path={:?}",
		config_path.as_ref()
	))?;
	let config: Config = toml::from_str(data.as_str()).context(format!("{:?}", config_path))?;
	let mut lock = CFG.write().await;
	*lock = config;
	Ok(())
}

#[derive(Deserialize, Default, Builder, Debug, Clone)]
#[builder(setter(into))]
pub struct Config {
	#[serde(default = "default_addr")]
	pub bind: String,
	#[serde(default = "default_log_directive")]
	pub log_directive: String,
	#[serde(default)]
	pub db_url: Option<String>,
}

fn default_addr() -> String {
	"0.0.0.0:8888".to_string()
}

fn default_log_directive() -> String {
	"info,tide=warn".to_string()
}

pub async fn cfg() -> async_std::sync::RwLockReadGuard<'static, Config> {
	return CFG.read().await;
}

/// Safely get a copy of the current config to avoid deadlocks
pub async fn get_config() -> Config {
	let config_guard = CFG.read().await;
	Config {
		bind: config_guard.bind.clone(),
		log_directive: config_guard.log_directive.clone(),
		db_url: config_guard.db_url.clone(),
	}
}

/// Safely get just the log directive to avoid deadlocks
pub async fn get_log_directive() -> String {
	let config_guard = CFG.read().await;
	config_guard.log_directive.clone()
}

/// Safely get just the database URL to avoid deadlocks
pub async fn get_db_url() -> Option<String> {
	let config_guard = CFG.read().await;
	config_guard.db_url.clone()
}
