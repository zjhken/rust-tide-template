use std::{
	sync::LazyLock,
	time::{Duration, Instant},
};

use dashmap::DashMap;
use serde::Deserialize;
use tracing::info;

use crate::server::make_resp;

static CRED_CACHE: LazyLock<DashMap<String, (String, Instant)>> = LazyLock::new(|| DashMap::new());

#[derive(Deserialize, Default, Debug, Clone)]
pub struct Cred {
	pub username: String,
	pub password: String,
}

pub struct AuthMiddleware;
#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> tide::Middleware<State> for AuthMiddleware {
	async fn handle(
		&self,
		req: tide::Request<State>,
		next: tide::Next<'_, State>,
	) -> tide::Result {
		info!("enter auth");
		let cred = read_cred_from_basic_auth(&req);
		match cred {
			None => return Ok(make_resp(401, "basic auth is required")),
			Some(cred) => {
				let mut authn_passed = false;
				if let Some(cached_pswd) = get_cached_cred(&cred.username).await {
					if cached_pswd == cred.password {
						authn_passed = true
					}
				} else if authn(&cred.username, &cred.password).await? {
					cache_cred(&cred.username, cred.password).await;
					authn_passed = true
				}
				if !authn_passed {
					return Ok(make_resp(401, "incorrect username or password"));
				}

				let resp = next.run(req).await;
				return Ok(resp);
			}
		}
	}
}

async fn authn(username: &str, password: &str) -> tide::Result<bool> {
	// let db = DB_POOL.get().unwrap();
	// let user = db.get_user_by_username(username).await?;
	// if let Some(user) = user {
	// 	if user.password == password {
	// 		return Ok(true);
	// 	}
	// }
	// Ok(false)
	Ok(true)
}

pub fn read_cred_from_basic_auth<State: Clone + Send + Sync + 'static>(
	req: &tide::Request<State>,
) -> Option<Cred> {
	let value = req.header("Authorization").or(req.header("authorization"));
	match value {
		Some(v) => {
			let s = v.as_str();
			let base64 = s.strip_prefix("Basic ");
			match base64 {
				None => return None,
				Some(s) => {
					let v = base64_simd::STANDARD.decode_to_vec(s).unwrap();
					let s = String::from_utf8_lossy(&v);
					let s: Vec<&str> = s.split(":").collect();
					let username = s.first().unwrap();
					let password = s.get(1).unwrap();
					return Some(Cred {
						username: (*username).to_owned(),
						password: (*password).to_owned(),
					});
				}
			}
		}
		None => return None,
	}
}

pub async fn cache_cred(username: &str, cred: String) {
	CRED_CACHE.insert(username.to_string(), (cred, Instant::now()));
}

const ONE_DAY: u64 = 60 * 60 * 24;

pub async fn get_cached_cred(username: &str) -> Option<String> {
	if let Some(x) = CRED_CACHE.get(username) {
		let timestamp = x.1;
		if timestamp.elapsed() < Duration::from_secs(ONE_DAY) {
			let pswd = &x.0;
			return Some(pswd.clone());
		} else {
			CRED_CACHE.remove(username);
		}
	}
	None
}
