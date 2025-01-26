mod auth;
mod cli;
mod common;
mod config;
mod database;
mod logging;
mod server;

use anyhow_ext::Result;
use clap::Parser;
use config::CFG;
use logging::setup_logger;
use server::init_http_server_blocking;
use tracing::info;

use crate::{cli::Cli, config::load_config, database::init_database};

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> Result<()> {
	async_std::task::block_on(async {
		let cli = Cli::parse();
		info!(?cli.config);
		// load_config(cli.config).await?;
		setup_logger(&CFG.read().await.log_level.0)?;
		// init_database(CFG.read().await.db_url.as_str())?;
		init_http_server_blocking()?;
		Ok(())
	})
}
