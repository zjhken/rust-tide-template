use core::fmt;
use std::{default, fmt::write, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
	#[arg(short, long)]
	pub env: Env,

	#[arg(short, long)]
	pub port: u16,
	/// set log level
	#[arg(short, long, action = clap::ArgAction::Count)]
	pub verbose: u8,

	/// Optional name to operate on
	#[arg(short, long, default_value = "config.toml")]
	pub db_url: String,

	/// Sets a custom config file
	#[arg(short, long, value_name = "FILE")]
	#[clap(default_value = "config.toml")]
	pub config: PathBuf,

	/// Turn debugging information on
	#[arg(short, long, action = clap::ArgAction::Count)]
	pub debug: u8,

	#[command(subcommand)]
	pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
	/// does testing things
	Test {
		/// lists test values
		#[arg(short, long)]
		list: bool,
	},
}

#[derive(Deserialize, ValueEnum, Clone, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Env {
	#[default]
	Dev,
	Uat,
	Prd,
}

impl fmt::Display for Env {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", format!("{:?}", self).to_lowercase())
	}
}
