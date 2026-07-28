use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(Todo::Table)
					.col(pk_auto(Todo::Id))
					.col(string(Todo::Title).char_len(256))
					.col(boolean(Todo::Done).not_null().default(false))
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(Todo::Table).to_owned())
			.await
	}
}

#[derive(Iden)]
enum Todo {
	Table,
	Id,
	Title,
	Done,
}
