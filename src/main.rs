mod auth;
mod cli;
mod config;
mod database;
mod fs_utils;
mod logger;
mod server;
mod utils;

use anyhow_ext::{Context, Result};
use clap::Parser;
use cli::Cli;
use server::init_http_server_blocking;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[async_std::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();

	// CLI 参数优先级最高
	config::load_config_from_cli(&cli).await.dot()?;

	// 如果指定了配置文件，则加载（会覆盖 CLI 参数中未指定的部分）
	config::load_config(cli.config.as_ref()).await.dot()?;

	logger::setup_logger().await.dot()?;

	// 初始化数据库（如果配置了 db_url）
	database::init_database(config::get_db_url().await.as_deref()).dot()?;

	init_http_server_blocking().await?;
	Ok(())
}
