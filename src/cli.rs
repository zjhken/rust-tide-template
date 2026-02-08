use core::fmt;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;

use crate::config::Config;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
	#[arg(short, long, default_value = "local")]
	pub env: Env,

	#[command(flatten)]
	pub config: Config,

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
