mod cli;
mod config;
mod database;
mod http_server;
mod logging;
mod utils;

use anyhow::Result;
use clap::Parser;
use http_server::init_http_server_blocking;

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
    load_config(cli.config.unwrap_or(".".into()))?;
    init_logger(&get_config().read().unwrap().log_level)?;
    init_database(get_config().read().unwrap().db_url.as_str())?;
    init_http_server_blocking()?;
    Ok(())
}
