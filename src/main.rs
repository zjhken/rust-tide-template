mod auth;
mod cli;
mod common;
mod config;
mod database;
// mod logging;
mod logger;
// mod log_api;
mod server;
mod test_logs;
mod utils;

use anyhow_ext::Result;
use clap::Parser;
use config::CFG;
use server::init_http_server_blocking;
use tracing::info;

use crate::{cli::Cli, config::{load_config, get_log_level}, database::init_database};

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[async_std::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();
	info!(?cli.config);

	// Use safe helper function to avoid deadlock
	let log_level = get_log_level().await;
	logger::setup_logger(&log_level)?;
	// load_config(cli.config).await?;
	// init_database(get_db_url().await.as_str())?;
	init_http_server_blocking()?;
	Ok(())
}
