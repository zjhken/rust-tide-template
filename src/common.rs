use anyhow_ext::anyhow;
use anyhow_ext::Context;
use anyhow_ext::Result;
use async_std::fs::OpenOptions;
use async_std::{fs, path::PathBuf};
use surf::Client;
use tracing::{error, info};

const A2Z: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

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

use std::{io::BufReader, sync::OnceLock, time::Duration};

pub(crate) use retry_http;

pub static HTTP_CLIENT: OnceLock<surf::Client> = OnceLock::new();

pub fn http() -> &'static Client {
	HTTP_CLIENT.get().unwrap()
}

pub fn setup_http_client(trust_store_bytes: Option<&[u8]>) -> Result<()> {
	let mut surf_config = surf::Config::new().set_timeout(Some(Duration::from_secs(10 * 60)));
	if let Some(pem_bytes) = trust_store_bytes {
		let mut tls_config = rustls_0181::ClientConfig::new();
		tls_config
			.root_store
			.add_pem_file(&mut BufReader::new(pem_bytes))
			.unwrap();
		let tls_config = std::sync::Arc::new(tls_config);
		surf_config = surf_config.set_tls_config(Some(tls_config));
	}
	let client: surf::Client = surf_config.try_into().unwrap();
	HTTP_CLIENT.get_or_init(|| client);
	Ok(())
}

pub async fn create_path_and_file_overwrite(path: &PathBuf) -> Result<fs::File> {
	fs::create_dir_all(
		path.parent()
			.ok_or(anyhow!("parent folder not exist. {:?}", path))?,
	)
	.await
	.dot()?;
	let file = OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.open(&path)
		.await
		.context(format!("failed to create file => {path:?}"))?;
	return Ok(file);
}

pub fn make_resp<S>(status: S, body: impl Into<tide::Body>) -> tide::Response
where
	S: TryInto<tide::StatusCode>,
	S::Error: std::fmt::Debug,
{
	let mut resp = tide::Response::new(status);
	resp.set_body(body);
	return resp;
}

pub struct ErrorHandleMiddleware;
#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> tide::Middleware<State> for ErrorHandleMiddleware {
	async fn handle(&self, req: tide::Request<State>, next: tide::Next<'_, State>) -> tide::Result {
		let mut resp = next.run(req).await;
		if resp.error().is_some() {
			let err = resp.error().unwrap();
			error!(?err);
			resp.set_body(format!("{:?}", err));
		}
		Ok(resp)
	}
}

pub(crate) async fn ensure_path(path: &std::path::PathBuf) -> Result<()> {
	info!(?path, "checking path");
	let path = async_std::path::PathBuf::from(path);
	if path.exists().await {
		info!(?path, "path already exists");
		return Ok(());
	}
	async_std::fs::create_dir(path).await.dot()?;
	Ok(())
}

pub async fn recursively_delete_empty_folder(
	dir: &async_std::path::Path,
	store_root: &async_std::path::PathBuf,
) -> Result<()> {
	let mut curr_dir = dir;
	loop {
		if curr_dir.is_dir().await {
			if curr_dir == store_root {
				break;
			} else {
				match fs::remove_dir(curr_dir).await {
					Err(err) => {
						let kind_str = err.kind().to_string();
						match kind_str.as_str() {
							"directory not empty" => {
								break;
							}
							other_kind => {
								let msg = err.to_string();
								return Err(anyhow!(
									"failed to delete a dir, err_kind={other_kind}, {msg}"
								));
							}
						}
					}
					Ok(_) => {
						curr_dir = curr_dir
							.parent()
							.ok_or(anyhow!("cannot get parent folder for {curr_dir:?}"))
							.dot()?
					}
				}
			}
		}
	}
	Ok(())
}

pub fn gen_n_random_str(n: u8) -> String {
	(0..n)
		.map(|_| {
			let idx = rand::random::<u8>() % (A2Z.len() as u8);
			A2Z.chars().nth(idx as usize).unwrap()
		})
		.collect()
}
