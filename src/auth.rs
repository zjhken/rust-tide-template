use std::{
	sync::LazyLock,
	time::{Duration, Instant},
};

use dashmap::DashMap;
use serde::Deserialize;
use tracing::info;

use crate::server::make_resp;

static CRED_CACHE: LazyLock<DashMap<String, (String, Instant)>> = LazyLock::new(DashMap::new);

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Cred {
	pub username: String,
	pub password: String,
}

pub struct AuthMiddleware;
#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> tide::Middleware<State> for AuthMiddleware {
	async fn handle(&self, req: tide::Request<State>, next: tide::Next<'_, State>) -> tide::Result {
		info!("enter auth");
		let Some(cred) = read_cred_from_basic_auth(&req) else {
			return Ok(make_resp(401, "basic auth is required"));
		};
		let mut authn_passed = false;
		if let Some(cached_pswd) = get_cached_cred(&cred.username) {
			if cached_pswd == cred.password {
				authn_passed = true;
			}
		} else if authn(&cred.username, &cred.password).await? {
			cache_cred(&cred.username, cred.password);
			authn_passed = true;
		}
		if !authn_passed {
			return Ok(make_resp(401, "incorrect username or password"));
		}
		Ok(next.run(req).await)
	}
}

// Stub: real DB lookup will be wired up when the smol-based framework replaces tide.
#[allow(clippy::unused_async)]
async fn authn(username: &str, password: &str) -> tide::Result<bool> {
	let _ = (username, password);
	Ok(true)
}

pub fn read_cred_from_basic_auth<State: Clone + Send + Sync + 'static>(
	req: &tide::Request<State>,
) -> Option<Cred> {
	let value = req
		.header("Authorization")
		.or_else(|| req.header("authorization"))?;
	let raw = value.as_str();
	let b64 = raw.strip_prefix("Basic ")?;
	let decoded = base64_simd::STANDARD.decode_to_vec(b64).ok()?;
	let decoded = String::from_utf8(decoded).ok()?;
	let (username, password) = decoded.split_once(':')?;
	Some(Cred {
		username: username.to_owned(),
		password: password.to_owned(),
	})
}

pub fn cache_cred(username: &str, cred: String) {
	CRED_CACHE.insert(username.to_string(), (cred, Instant::now()));
}

const ONE_DAY: u64 = 60 * 60 * 24;

pub fn get_cached_cred(username: &str) -> Option<String> {
	if let Some(x) = CRED_CACHE.get(username) {
		let timestamp = x.1;
		if timestamp.elapsed() < Duration::from_secs(ONE_DAY) {
			let pswd = &x.0;
			return Some(pswd.clone());
		}
		CRED_CACHE.remove(username);
	}
	None
}
