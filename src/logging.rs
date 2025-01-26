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

use std::time::{Duration, Instant};

use surf::StatusCode;
use tide::Response;
use tide::{Middleware, Next, Request};
use tracing::{debug, error, error_span, info, info_span, warn, warn_span, Instrument};

// TODO: use fastrace https://github.com/fast/fastrace/blob/main/fastrace/examples/asynchronous.rs#L13

async_std::task_local! {
	static REQ_ID: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

pub fn setup_logger(log_level: &Level) -> Result<()> {
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
					"{timestamp}|{level}|{req_id}|{message}",
					timestamp = humantime::format_rfc3339_millis(SystemTime::now()),
					level = record.level(),
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
		.filter(|metadata| !metadata.target().starts_with("tide::log::middleware"))
		.chain(std::io::stderr())
		.apply()?;
	Ok(())
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
