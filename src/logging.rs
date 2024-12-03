use std::{
	fmt::Debug,
	sync::{Mutex, OnceLock},
};

use anyhow_ext::Context;
use anyhow_ext::{Ok, Result};
use tracing::{info, level_filters::LevelFilter, Level};

use crate::config::LogLevel;
use tracing_subscriber::{
	filter,
	fmt::{self, layer},
	prelude::*,
	reload::{self, Handle},
	Registry,
};

// TODO: use fastrace https://github.com/fast/fastrace/blob/main/fastrace/examples/asynchronous.rs#L13

static RELOAD_HANDLE: OnceLock<Mutex<Handle<LevelFilter, Registry>>> = OnceLock::new();
pub fn init_logger(log_level: &LogLevel) -> Result<()> {
	let filter = convert_to_level_filter_level(log_level);
	let (filter, reload_handle) = reload::Layer::new(filter);
	RELOAD_HANDLE.get_or_init(|| Mutex::new(reload_handle));

	let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
	tracing_subscriber::registry()
		.with(stderr_layer.with_filter(filter))
		.with(tracing_subscriber::fmt::layer().pretty())
		.init();

	Ok(())
}

pub fn init_logger_remove_tide_log(log_level: &LogLevel) -> Result<()> {
	let filter = convert_to_level_filter_level(log_level);
	let (filter, reload_handle) = reload::Layer::new(filter);
	RELOAD_HANDLE.get_or_init(|| Mutex::new(reload_handle));

	let filter_out_tide_log = tracing_subscriber::filter::filter_fn(|metadata| {
		!metadata.target().starts_with("tide::log::middleware")
	});
	let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
	tracing_subscriber::registry()
		.with(stderr_layer.with_filter(filter))
		.with(filter_out_tide_log)
		.init();
	Ok(())
}

pub fn reload_log_level(log_level: &LogLevel) -> Result<()> {
	let new_filter = convert_to_level_filter_level(log_level);
	let rh = RELOAD_HANDLE.get().unwrap().lock().unwrap();
	(*rh).modify(|filter| *filter = new_filter)?;
	Ok(())
}

pub fn init_logger_old(log_level: &LogLevel) -> Result<()> {
	let tracing_log_level = convert_to_tracing_log_level(log_level);

	let subscriber = tracing_subscriber::FmtSubscriber::builder()
		.with_writer(std::io::stderr)
		.with_max_level(tracing_log_level)
		.finish();
	tracing::subscriber::set_global_default(subscriber)?;

	// let filter = convert_to_level_filter_level(log_level);
	// let (filter, reload_handle) =
	//     reload::Layer::<tracing::level_filters::LevelFilter, Registry>::new(filter);

	// let registry = Registry::default();
	// let layer = layer().with_writer(std::io::stderr);
	// registry.with(layer).with(filter).init();

	Ok(())
}

fn convert_to_level_filter_level(log_level: &LogLevel) -> LevelFilter {
	return match log_level {
		LogLevel::Debug => filter::LevelFilter::DEBUG,
		LogLevel::Info => filter::LevelFilter::INFO,
		LogLevel::Warn => filter::LevelFilter::WARN,
		LogLevel::Trace => filter::LevelFilter::TRACE,
	};
}

fn convert_to_tracing_log_level(log_level: &LogLevel) -> Level {
	return match log_level {
		LogLevel::Debug => tracing::Level::DEBUG,
		LogLevel::Info => tracing::Level::INFO,
		LogLevel::Warn => tracing::Level::WARN,
		LogLevel::Trace => tracing::Level::TRACE,
	};
}
