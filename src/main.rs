mod auth;
mod cli;
mod config;
mod database;
mod entity;
mod logger;
mod server;
mod utils;

use anyhow_ext::{Context, Result};
use clap::Parser;
use cli::Cli;
use server::init_http_server_blocking;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[async_std::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();

	config::load_config(cli.config, cli.config_file.as_deref())
		.await
		.dot()?;

	logger::setup_logger().await.dot()?;

	database::init_database(config::cfg().await.db_url.clone().as_deref()).dot()?;

	init_http_server_blocking().await?;
	Ok(())
}
