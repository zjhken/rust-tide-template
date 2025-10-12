use std::time::SystemTime;
use std::{fmt::Debug, sync::OnceLock};

use async_std::sync::Mutex;

use anyhow_ext::Context;
use anyhow_ext::Result;
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::fmt::format;

use crate::auth;
use crate::common::gen_n_random_str;
use crate::config::LogLevel;
use tracing_subscriber::{
	filter,
	fmt::{self, layer},
	prelude::*,
	reload::{self, Handle},
	Registry,
};

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use surf::StatusCode;
use tide::Response;
use tide::{Middleware, Next, Request};
use tracing::{debug, error, error_span, info, info_span, warn, warn_span, Instrument};

// TODO: use fastrace https://github.com/fast/fastrace/blob/main/fastrace/examples/asynchronous.rs#L13

static TARGET_LEVELS: OnceLock<Arc<DashMap<String, log::LevelFilter>>> = OnceLock::new();

async_std::task_local! {
	static REQ_ID: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

pub fn setup_logger(log_level: &Level) -> Result<()> {
	// Initialize target levels storage
	let target_levels = Arc::new(DashMap::new());
	TARGET_LEVELS
		.set(target_levels.clone())
		.map_err(|_| anyhow_ext::anyhow!("Failed to set target levels"))?;

	fern::Dispatch::new()
		.format(|out, message, record| {
			let mut req_id = String::new();
			REQ_ID.with(|id| req_id = id.borrow().to_string());
			if req_id.is_empty() {
				out.finish(format_args!(
					"{timestamp}|{level}|{target}|{message}",
					timestamp = humantime::format_rfc3339_millis(SystemTime::now()),
					level = record.level(),
					target = record.target(),
				));
			} else if record.level() == log::Level::Error {
				out.finish(format_args!(
					"{timestamp}|{level}|{target}|{req_id}|{file}@{line}|{message}",
					timestamp = humantime::format_rfc3339_millis(SystemTime::now()),
					level = record.level(),
					target = record.target(),
					file = record.file().unwrap_or("unknown_file"),
					line = record.line().unwrap_or(0),
				));
			} else {
				out.finish(format_args!(
					"{timestamp}|{level}|{target}|{req_id}|{message}",
					timestamp = humantime::format_rfc3339_millis(SystemTime::now()),
					level = record.level(),
					target = record.target(),
				));
			}
		})
		.level(match log_level {
			&tracing::Level::ERROR => log::LevelFilter::Error,
			&tracing::Level::WARN => log::LevelFilter::Warn,
			&tracing::Level::INFO => log::LevelFilter::Info,
			&tracing::Level::DEBUG => log::LevelFilter::Debug,
			&tracing::Level::TRACE => log::LevelFilter::Trace,
		})
		.filter(|metadata| {
			// Filter out tide middleware logs
			if metadata.target().starts_with("tide::log::middleware") {
				return false;
			}

			// Check if we have a specific level for this target
			if let Some(target_levels) = TARGET_LEVELS.get() {
				if let Some(level_filter) = target_levels.get(metadata.target()) {
					return metadata.level() <= *level_filter;
				}
			}
			true
		})
		.chain(std::io::stderr())
		.apply()?;
	Ok(())
}

pub fn set_target_log_level(target: String, level: log::LevelFilter) -> Result<()> {
	if let Some(target_levels) = TARGET_LEVELS.get() {
		target_levels.insert(target.clone(), level);
		info!("Set log level for target '{}' to {:?}", target, level);
		Ok(())
	} else {
		Err(anyhow_ext::anyhow!("Target levels not initialized"))
	}
}

pub fn get_target_log_level(target: &str) -> Option<log::LevelFilter> {
	TARGET_LEVELS
		.get()
		.and_then(|target_levels| target_levels.get(target).map(|entry| *entry))
}

pub fn remove_target_log_level(target: &str) -> Result<bool> {
	if let Some(target_levels) = TARGET_LEVELS.get() {
		Ok(target_levels.remove(target).is_some())
	} else {
		Err(anyhow_ext::anyhow!("Target levels not initialized"))
	}
}

pub fn list_target_log_levels() -> Vec<(String, log::LevelFilter)> {
	if let Some(target_levels) = TARGET_LEVELS.get() {
		target_levels
			.iter()
			.map(|entry| (entry.key().clone(), *entry.value()))
			.collect()
	} else {
		Vec::new()
	}
}

#[derive(Debug, Default, Clone)]
pub struct AccessLogMiddleware;
impl AccessLogMiddleware {}
#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> tide::Middleware<State> for AccessLogMiddleware {
	async fn handle(&self, req: tide::Request<State>, next: tide::Next<'_, State>) -> tide::Result {
		let path = req.url().path().to_owned();
		let method = req.method();
		let ip = req.peer_addr().unwrap_or("-").to_string();
		let username = auth::read_cred_from_basic_auth(&req)
			.map(|cred| cred.username)
			.unwrap_or("-".to_owned());
		REQ_ID.with(|id| {
			let mut id = id.borrow_mut();
			id.clear();
			id.push_str(gen_n_random_str(6).as_str());
		});
		let agent = req.header("user-agent").map(|a| a.as_str()).unwrap_or("-");
		let agent = agent
			.split_once(' ')
			.map(|(a, _)| a)
			.unwrap_or("-")
			.to_string();
		let req_body_size = req.len().unwrap_or(0);

		let start = Instant::now();

		let response = next.run(req).await;

		let size = match method {
			tide::http::Method::Post => req_body_size,
			_ => response.len().unwrap_or(0),
		};
		let duration = start.elapsed();
		let status = response.status();

		let access_log_msg =
			format!("{ip}|{agent}|{username}|{method}|{status}|{duration:?}|{size}B|{path}",);
		info!("{access_log_msg}");

		return Ok(response);
	}
}
