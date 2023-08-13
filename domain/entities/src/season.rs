//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.1

use sea_orm::entity::prelude::*;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
	fn table_name(&self) -> &str {
		"season"
	}
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq)]
pub struct Model {
	pub id: Uuid,
	pub series_id: Uuid,
	pub number: i16,
	pub name: Option<String>,
	pub tvdb_id: i32,
	pub theme_song_id: Option<Uuid>,
	pub version: TimeDateTime,
	pub image: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
	Id,
	SeriesId,
	Number,
	Name,
	TvdbId,
	ThemeSongId,
	#[sea_orm(column_name = "_version")]
	Version,
	Image,
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
	Series,
	ThemeSong,
}

impl ColumnTrait for Column {
	type EntityName = Entity;
	fn def(&self) -> ColumnDef {
		match self {
			Self::Id => ColumnType::Uuid.def(),
			Self::SeriesId => ColumnType::Uuid.def(),
			Self::Number => ColumnType::SmallInteger.def(),
			Self::Name => ColumnType::String(None).def().null(),
			Self::TvdbId => ColumnType::Integer.def(),
			Self::ThemeSongId => ColumnType::Uuid.def().null(),
			Self::Version => ColumnType::DateTime.def(),
			Self::Image => ColumnType::String(None).def().null(),
		}
	}
}

impl RelationTrait for Relation {
	fn def(&self) -> RelationDef {
		match self {
			Self::Series => Entity::belongs_to(super::series::Entity)
				.from(Column::SeriesId)
				.to(super::series::Column::Id)
				.into(),
			Self::ThemeSong => Entity::belongs_to(super::theme_song::Entity)
				.from(Column::ThemeSongId)
				.to(super::theme_song::Column::Id)
				.into(),
		}
	}
}

impl Related<super::series::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Series.def()
	}
}

impl Related<super::theme_song::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::ThemeSong.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
