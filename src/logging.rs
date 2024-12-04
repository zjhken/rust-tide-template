use std::{fmt::Debug, sync::OnceLock};

use async_std::sync::Mutex;

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
pub fn init_logger(log_level: &Level) -> Result<()> {
	let target_filter_layer = filter::Targets::new()
		.with_targets(vec![
			("async_io::driver", Level::TRACE),
			("tide", Level::DEBUG),
			("excel_to_web", Level::DEBUG),
		])
		.with_target("tide::log::middleware", LevelFilter::OFF)
		.with_target("polling::epoll", LevelFilter::OFF);

	let level: LevelFilter = log_level.to_owned().into();
	let (level_filter, reload_handle) = reload::Layer::new(level);
	RELOAD_HANDLE.get_or_init(|| Mutex::new(reload_handle));

	tracing_subscriber::registry()
		.with(
			tracing_subscriber::fmt::layer()
				.compact()
				.with_writer(std::io::stderr)
				.with_filter(level_filter), // .with_filter(filter),
		)
		// .with(tracing_subscriber::fmt::layer().compact()) // this will add one more output, by default is stdout
		// .with(log_level_filter_layer)
		.with(target_filter_layer)
		.init();

	Ok(())
}

pub async fn reload_log_level(log_level: impl Into<LevelFilter>) -> Result<()> {
	RELOAD_HANDLE
		.get()
		.unwrap()
		.lock()
		.await
		.reload(log_level)
		.dot()?;
	Ok(())
}

pub fn init_logger_old(log_level: &LogLevel) -> Result<()> {
	let subscriber = tracing_subscriber::FmtSubscriber::builder()
		.with_writer(std::io::stderr)
		.with_max_level(log_level.0)
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
