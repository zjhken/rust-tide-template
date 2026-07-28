use sea_orm::*;

use crate::entity::todo;

pub async fn list(db: &DatabaseConnection) -> Result<Vec<todo::Model>, DbErr> {
	todo::Entity::find().all(db).await
}

pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<todo::Model>, DbErr> {
	todo::Entity::find_by_id(id).one(db).await
}

pub async fn create(
	db: &DatabaseConnection, title: String, done: bool,
) -> Result<todo::Model, DbErr> {
	todo::ActiveModel {
		title: Set(title),
		done: Set(done),
		..Default::default()
	}
	.insert(db)
	.await
}

pub async fn update(
	db: &DatabaseConnection, id: i32, title: Option<String>, done: Option<bool>,
) -> Result<todo::Model, DbErr> {
	let model: todo::ActiveModel = todo::Entity::find_by_id(id)
		.one(db)
		.await?
		.ok_or(DbErr::RecordNotFound("todo".to_string()))?
		.into();
	let model = todo::ActiveModel {
		id: model.id,
		title: title.map(Set).unwrap_or(model.title),
		done: done.map(Set).unwrap_or(model.done),
	};
	model.update(db).await
}

pub async fn delete_by_id(db: &DatabaseConnection, id: i32) -> Result<u64, DbErr> {
	let res = todo::Entity::delete_by_id(id).exec(db).await?;
	Ok(res.rows_affected)
}
