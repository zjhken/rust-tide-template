use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "todo")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i32,
	#[sea_orm(column_type = "Text", column_name = "title")]
	pub title: String,
	pub done: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
