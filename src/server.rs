use std::time::{Duration, Instant};

use anyhow_ext::{Context, Result, anyhow};
use surf::StatusCode;
use tide::Response;
use tide::{Middleware, Next, Request};
use tracing::{Instrument, debug, error, error_span, info, info_span, warn, warn_span};

use crate::{auth, logger, utils};
// use crate::log_api::{handle_set_log_level, handle_get_log_level, handle_delete_log_level, handle_list_log_levels, handle_generate_test_logs};

const token: u32 = 0x60db1e55;

pub fn init_http_server_blocking() -> Result<()> {
	let mut app = tide::new();
	app.with(ErrorHandleMiddleware {});
	app.with(CorsMiddleware {});
	app.with(AuthMiddleware {});
	app.with(AccessLogMiddleware {});

	app.with(AuthMiddleware {});

	app.at("/")
		.get(|_| async move { Ok("this is a inline handler") });
	app.at("/user/:name").get(example_handler);

	// Log level management routes
	app.at("/api/log/:directive")
		.post(async |req: Request<()>| {
			let directive = req
				.param("directive")
				.map_err(|_e| anyhow!("directive is required"))
				.dot()?;
			logger::update_global_log_level(directive).dot()?;
			Ok(make_resp(200, ""))
		})
		.get(async |_req| {
			Ok(make_resp(200, logger::get_global_log_level().dot()?))
		});

	async_std::task::block_on(async {
		app.listen("0.0.0.0:8888").await?;
		Ok(())
	})
}

async fn example_handler(req: Request<()>) -> tide::Result<Response> {
	// 测试 nested span
	let outer_span = info_span!("example_handler", name = "test_user");
	async move {
		info!("进入 example_handler");

		// 第一个嵌套 span
		let span1 = info_span!("validation", step = "validate_input");
		async {
			info!("验证用户输入");
			debug!("输入参数检查完成");
		}
		.instrument(span1)
		.await;

		// 第二个嵌套 span
		let span2 = info_span!("business_logic", step = "process_data");
		async {
			info!("执行业务逻辑");
			debug!("数据处理中...");

			// 第三层嵌套
			let span3 = info_span!("database", operation = "query");
			async {
				info!("查询数据库");
				debug!("SQL 执行完成");
			}
			.instrument(span3)
			.await;
		}
		.instrument(span2)
		.await;

		info!("example_handler 完成");
		Ok(make_resp(StatusCode::Ok, "nested span test"))
	}
	.instrument(outer_span)
	.await
}

struct AuthMiddleware;
#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for AuthMiddleware {
	async fn handle(&self, mut req: Request<State>, next: Next<'_, State>) -> tide::Result {
		debug!("request counter");
		// req.set_ext(RequestCount(count));

		let mut res = next.run(req).await;

		// res.insert_header("request-number", count.to_string());
		Ok(res)
	}
}

pub fn make_resp<S>(status: S, body: impl Into<tide::Body>) -> Response
where
	S: TryInto<tide::StatusCode>,
	S::Error: std::fmt::Debug,
{
	let mut resp = Response::new(status);
	resp.set_body(body);
	return resp;
}

struct ErrorHandleMiddleware;
#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for ErrorHandleMiddleware {
	async fn handle(&self, mut req: Request<State>, next: Next<'_, State>) -> tide::Result {
		let mut resp = next.run(req).await;
		if let Some(err) = resp.error() {
			error!(?err);
			resp.set_body(format!("{err:?}"));
		}
		Ok(resp)
	}
}

struct CorsMiddleware;
#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for CorsMiddleware {
	async fn handle(&self, mut req: Request<State>, next: Next<'_, State>) -> tide::Result {
		let mut resp = match req.method() {
			surf::http::Method::Options => make_resp(200, ""),
			_ => next.run(req).await,
		};
		resp.insert_header("Access-Control-Allow-Origin", "http://localhost:5173");
		resp.insert_header(
			"Access-Control-Allow-Headers",
			"Origin, X-Requested-With, Content-Type, Accept",
		);
		resp.insert_header(
			"Access-Control-Allow-Methods",
			"GET, POST, PUT, DELETE, OPTIONS",
		);
		resp.insert_header("Access-Control-Max-Age", "7200 "); // reduce OPTIONS requests. 7200 is Chrome maximum number
		resp.insert_header("Access-Control-Allow-Credentials", "true"); // reduce OPTIONS requests
		return Ok(resp);
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
		utils::set_req_id();
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
