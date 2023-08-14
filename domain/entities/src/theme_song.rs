//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.1

use sea_orm::entity::prelude::*;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
	fn table_name(&self) -> &str {
		"theme_song"
	}
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq)]
pub struct Model {
	pub id: Uuid,
	pub name: String,
	pub youtube_id: Option<String>,
	pub youtube_starts_at: Option<i32>,
	pub youtube_ends_at: Option<i32>,
	pub version: TimeDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
	Id,
	Name,
	YoutubeId,
	YoutubeStartsAt,
	YoutubeEndsAt,
	#[sea_orm(column_name = "_version")]
	Version,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
	Id,
}

impl PrimaryKeyTrait for PrimaryKey {
	type ValueType = Uuid;
	fn auto_increment() -> bool {
		false
	}
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
	Season,
	Series,
}

impl ColumnTrait for Column {
	type EntityName = Entity;
	fn def(&self) -> ColumnDef {
		match self {
			Self::Id => ColumnType::Uuid.def(),
			Self::Name => ColumnType::String(None).def(),
			Self::YoutubeId => ColumnType::String(None).def().null(),
			Self::YoutubeStartsAt => ColumnType::Integer.def().null(),
			Self::YoutubeEndsAt => ColumnType::Integer.def().null(),
			Self::Version => ColumnType::DateTime.def(),
		}
	}
}

impl RelationTrait for Relation {
	fn def(&self) -> RelationDef {
		match self {
			Self::Season => Entity::has_many(super::season::Entity).into(),
			Self::Series => Entity::has_many(super::series::Entity).into(),
		}
	}
}

impl Related<super::season::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Season.def()
	}
}

impl Related<super::series::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Series.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}