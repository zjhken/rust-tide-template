use core::fmt;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
	#[arg(short, long, default_value = "local")]
	pub env: Env,

	/// Server address (e.g., "0.0.0.0:8888")
	#[arg(short, long, default_value = "0.0.0.0:8888")]
	pub bind: String,

	/// set log level
	#[arg(short, long, action = clap::ArgAction::Count)]
	pub verbose: u8,

	/// Database URL (optional)
	#[arg(short, long)]
	pub db_url: Option<String>,

	/// Sets a custom config file (optional)
	#[arg(short, long, value_name = "FILE")]
	pub config: Option<PathBuf>,

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
	Local,
	Uat,
	Prd,
}

impl fmt::Display for Env {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", format!("{:?}", self).to_lowercase())
	}
}
