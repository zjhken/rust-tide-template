use std::time::{Duration, Instant};

use anyhow_ext::Result;
use surf::StatusCode;
use tide::Response;
use tide::{Middleware, Next, Request};
use tracing::{debug, error, error_span, info, info_span, warn, warn_span, Instrument};

use crate::logging::AccessLogMiddleware;

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

	async_std::task::block_on(async {
		app.listen("0.0.0.0:8888").await?;
		Ok(())
	})
}

async fn example_handler(req: Request<()>) -> tide::Result<Response> {
	// cannot use any number as status code
	Ok(make_resp(StatusCode::Ok, ""))
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
