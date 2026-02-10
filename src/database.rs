use std::sync::OnceLock;

use anyhow_ext::{Context, Result};
use sea_orm::{Database, DbConn};
use sea_orm_migration::MigratorTrait;
use tracing::info;

use migration::Migrator;

pub type DatabaseConnection = DbConn;

static DB_CONN: OnceLock<DatabaseConnection> = OnceLock::new();

/// Initialize the database connection and run migrations
pub fn init_database(db_url: Option<&str>) -> Result<()> {
	let db_url = match db_url {
		Some(url) => url,
		None => {
			tracing::info!("No database URL configured, skipping database initialization");
			return Ok(());
		}
	};

	return async_std::task::block_on(async {
		// Create database file if not exists
		ensure_db_file(db_url).await?;

		// Connect to database
		let db = Database::connect(db_url)
			.await
			.context("failed to connect to database")?;

		// Run migrations
		Migrator::up(&db, None)
			.await
			.context("failed to run migrations")?;

		info!("Database initialized and migrations completed");

		DB_CONN.set(db).expect("Failed to set global DB connection");

		Ok(())
	})
}

/// Get the global database connection
pub fn get_db_conn() -> &'static DatabaseConnection {
	DB_CONN.get().expect("Database not initialized")
}

async fn ensure_db_file(db_url: &str) -> Result<()> {
	if let Some((_, path)) = db_url.split_once("//") {
		if !async_std::path::Path::new(path).exists().await {
			info!("Creating database file {}", path);
			async_std::fs::File::create(path).await?;
		}
	}
	Ok(())
}
