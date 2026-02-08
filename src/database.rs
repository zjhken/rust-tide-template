use std::sync::OnceLock;

use anyhow_ext::{anyhow, Ok, Result};
use sea_query::{
	ColumnDef, Expr, Iden, QueryStatementWriter, SqliteQueryBuilder, Table, Value,
};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use tracing::info;

static DB_POOL: OnceLock<Pool<Sqlite>> = OnceLock::new();

pub fn init_db_pool(pool: Pool<Sqlite>) -> &'static Pool<Sqlite> {
	DB_POOL.get_or_init(|| pool)
}

pub fn get_db_pool() -> &'static Pool<Sqlite> {
	DB_POOL.get().unwrap()
}

pub fn init_database(db_url: Option<&str>) -> Result<()> {
	let db_url = match db_url {
		Some(url) => url,
		None => {
			tracing::info!("No database URL configured, skipping database initialization");
			return Ok(());
		}
	};

	async_std::task::block_on(async {
		ensure_db_file(db_url).await?;
		let pool = SqlitePoolOptions::new().connect(db_url).await?;
		let mut _pool = init_db_pool(pool);
		ensure_tables().await?;
		Ok(())
	})
}

async fn ensure_db_file(db_url: &str) -> Result<()> {
	if let Some((_, path)) = db_url.split_once("//") {
		if !std::path::Path::new(path).exists() {
			info!("Creating database file {}", path);
			async_std::fs::File::create(path).await?;
		} else {
			let metadata = async_std::fs::metadata(path).await?;
			if !metadata.is_file() {
				return Err(anyhow!("Database path is not a file. path={}", path));
			}
		}
	} else {
		return Err(anyhow!("failed to split db url string, db_url={}", db_url));
	}
	return Ok(());
}

async fn ensure_tables() -> Result<()> {
	// TODO: use sea_query
	let pool = get_db_pool();

	let sql = Table::create()
		.table(User::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(User::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(User::Age).integer().not_null())
		.col(ColumnDef::new(User::Username).string().not_null())
		.col(ColumnDef::new(User::Password).string().not_null())
		.col(ColumnDef::new(User::Id).uuid().default(Value::Int(None)))
		// .foreign_key(
		//     ForeignKey::create()
		//         .name("FK_2e303c3a712662f1fc2a4d0aad6")
		//         .from(User::Table, User::FontId)
		//         .to(Font::Table, Font::Id)
		//         .on_delete(ForeignKeyAction::Cascade)
		//         .on_update(ForeignKeyAction::Cascade)
		// )
		.to_string(SqliteQueryBuilder);

	let sql = sea_query::Query::select()
		.column(User::Username)
		.from(User::Table)
		.and_where(
			Expr::expr(Expr::col(User::Age).add(1))
				.mul(2)
				.eq(Expr::expr(Expr::col(User::Age).div(2)).sub(1)),
		)
		.and_where(
			Expr::col(User::Username).in_subquery(
				sea_query::Query::select()
					.expr(Expr::cust_with_values("ln($1 ^ $2)", [2.4, 1.2]))
					.take(),
			),
		)
		.and_where(
			Expr::col(User::Password)
				.like("D")
				.and(Expr::col(User::Password).like("E")),
		)
		.to_string(SqliteQueryBuilder);

	let result = sqlx::query(&sql).bind(&sql).execute(get_db_pool()).await?;

	todo!("implement your table logic");
	Ok(())
}

#[derive(Iden)]
enum User {
	Table,
	Id,
	Username,
	Password,
	Age,
}
