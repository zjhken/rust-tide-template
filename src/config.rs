use std::{path::Path, sync::OnceLock};

use async_std::sync::RwLock;

use anyhow_ext::Context;
use anyhow_ext::Result;
use serde::Deserialize;
use std::fmt::Debug;

use crate::cli::Cli;

static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();

pub fn load_from_cli(cli: &Cli) -> Result<()> {
	let config = Config {
		port: cli.port,
		log_level: match cli.verbose {
			0 => LogLevel::Info,
			1 => LogLevel::Debug,
			2.. => LogLevel::Trace,
		},
		db_url: cli.db_url.to_owned(),
	};
	init_config(config);
	Ok(())
}

pub fn load_config<P>(config_path: P) -> Result<()>
where
	P: AsRef<Path> + Debug,
{
	let data = std::fs::read_to_string(&config_path).context(format!(
		"failed to read config file data, path={:?}",
		config_path.as_ref()
	))?;
	let config: Config = toml::from_str(data.as_str()).context(format!("{:?}", config_path))?;
	init_config(config);
	Ok(())
}

pub fn init_config(config: Config) -> &'static RwLock<Config> {
	CONFIG.get_or_init(|| RwLock::new(config))
}

pub async fn cfg() -> async_std::sync::RwLockReadGuard<'static, Config> {
	CONFIG.get().unwrap().read().await
}

#[derive(Deserialize, Default)]
pub struct Config {
	pub port: u16,
	pub log_level: LogLevel,
	pub db_url: String,
}

#[derive(Deserialize, Default)]
pub enum LogLevel {
	#[default]
	Debug,
	Info,
	Warn,
	Trace,
}
