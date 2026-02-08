


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
use anyhow_ext::anyhow;
use anyhow_ext::Context;
use anyhow_ext::Result;
use async_std::fs::OpenOptions;
use async_std::{fs, path::PathBuf};
use tracing::info;


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