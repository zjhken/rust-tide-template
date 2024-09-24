use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
	/// set log level
	#[arg(short, long, action = clap::ArgAction::Count)]
	pub verbose: u8,

	/// Optional name to operate on
	pub name: Option<String>,

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
