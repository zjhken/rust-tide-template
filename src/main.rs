#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(
	clippy::module_name_repetitions,
	clippy::must_use_candidate,
	clippy::missing_errors_doc
)]
// Temporary: dead_code exists because tide-specific parts (auth, todo_repo, handlers)
// are being skipped pending the smol-based framework rewrite. Remove this allow after.
#![allow(dead_code)]

mod auth;
mod cli;
mod config;
mod database;
mod entity;
mod logger;
mod repository;
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
