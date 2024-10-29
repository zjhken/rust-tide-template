use anyhow_ext::Result;
use surf::Client;

macro_rules! retry_http {
	($f:expr, $maxTries:expr, $interval:expr, $retry_http_codes:expr) => {{
		let mut tries = 0;
		let result = loop {
			let result = $f;
			tries += 1;
			match result {
				Ok(ref resp) => {
					let status = resp.status();
					if ($retry_http_codes.contains(&(status as u16))) {
						tracing::warn!(
							"({}/{}) retry: bad status code. {}",
							tries,
							$maxTries,
							status
						);
						if tries >= $maxTries {
							tracing::error!("exceed maxTries");
							break result;
						}
						async_std::task::sleep(std::time::Duration::from_millis($interval)).await
					} else {
						break result;
					}
				}
				Err(ref e) => {
					tracing::warn!("({}/{}) retry: error. {}", tries, $maxTries, e);
					if tries >= $maxTries {
						tracing::error!("exceed maxTries");
						break result;
					}
					async_std::task::sleep(std::time::Duration::from_millis($interval)).await
				}
			}
		};
		result
	}};
}

use std::sync::OnceLock;

pub(crate) use retry_http;

pub static HTTP_CLIENT: OnceLock<surf::Client> = OnceLock::new();

pub fn http() -> &'static Client {
	HTTP_CLIENT.get().unwrap()
}

pub fn setup_http_client() -> Result<()> {
	let surf_config = surf::Config::new();
	let client: surf::Client = surf_config.try_into().unwrap();
	HTTP_CLIENT.get_or_init(|| client);
	Ok(())
}
