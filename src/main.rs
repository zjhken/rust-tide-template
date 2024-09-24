mod cli;
mod common;
mod config;
mod database;
mod http_server;
mod logging;

use anyhow_ext::Result;
use clap::Parser;
use http_server::init_http_server_blocking;
use tracing::info;

use crate::{
	cli::Cli,
	config::{get_config, load_config},
	database::init_database,
	logging::init_logger,
};

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> Result<()> {
	let cli = Cli::parse();
	info!(?cli.config);
	load_config(cli.config)?;
	init_logger(&get_config().read().unwrap().log_level)?;
	init_database(get_config().read().unwrap().db_url.as_str())?;
	init_http_server_blocking()?;
	Ok(())
}
