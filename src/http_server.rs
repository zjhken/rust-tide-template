use anyhow::Result;
use tide::{Middleware, Next, Request};
use tracing::debug;

pub fn init_http_server_blocking() -> Result<()> {
    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());

    app.with(AuthMiddleware {});

    app.at("/").get(|_| async { Ok("Hello, world!") });
    app.at("/user/{}").get(|_| async { Ok("Hello, world!") });

    async_std::task::block_on(async {
        app.listen("127.0.0.1:8080").await?;
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
