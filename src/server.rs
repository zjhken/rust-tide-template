use std::time::{Duration, Instant};

use anyhow_ext::Result;
use tide::{Middleware, Next, Request};
use tracing::{debug, error_span, info, info_span, warn, warn_span, Instrument};

pub fn init_http_server_blocking() -> Result<()> {
	let mut app = tide::new();
	app.with(tide::log::LogMiddleware::new());

	app.with(AuthMiddleware {});

	app.at("/").get(|_| async { Ok("Hello, world!") });
	app.at("/user/:name").get(|_| async { Ok("Hello, world!") });

	async_std::task::block_on(async {
		app.listen("0.0.0.0:8888").await?;
		Ok(())
	})
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

#[derive(Debug, Default, Clone)]
pub struct AccessLogMiddleware;
impl AccessLogMiddleware {
	async fn log<'a, State: Clone + Send + Sync + 'static>(
		&'a self,
		ctx: Request<State>,
		next: Next<'a, State>,
	) -> tide::Result {
		let path = ctx.url().path().to_owned();
		let method = ctx.method();

		Ok(async {
			let start = Instant::now();
			let response = next.run(ctx).await;
			let duration = start.elapsed();
			let status = response.status();

			info_span!("Response", http.code = status as u16, http.duration = ?duration).in_scope(
				|| {
					if status.is_server_error() {
						let span = error_span!(
							"internal error",
							detail = tracing::field::Empty,
							error = tracing::field::Empty
						);
						if let Some(err) = response.error() {
							span.record("error", tracing::field::display(err));
							span.record("detail", tracing::field::debug(err));
						}
					} else if status.is_client_error() {
						warn_span!("client error").in_scope(|| warn!("sent"));
					} else {
						info!("sent")
					}
				},
			);
			response
		}
		.instrument({
			let span = info_span!("request", req_id = tracing::field::Empty, method = %method, path = %path);
			span.record("req_id", rusty_ulid::Ulid::generate().to_string());
			span
		})
		.await)
	}
}
