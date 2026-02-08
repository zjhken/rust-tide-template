mod auth;
mod cli;
mod config;
mod database;
mod fs_utils;
mod logger;
mod server;
mod utils;

use anyhow_ext::{Context, Result};
use server::init_http_server_blocking;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[async_std::main]
async fn main() -> Result<()> {
	// let cli = Cli::parse();
	// info!(?cli.config);
	config::load_config("config.example.toml").await.dot()?;
	// Use safe helper function to avoid deadlock
	let log_level = config::get_log_level().await;
	logger::setup_logger(&log_level).dot()?;

	// init_database(get_db_url().await.as_str())?;
	init_http_server_blocking()?;
	Ok(())
}
