//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.1

use sea_orm::entity::prelude::*;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
	fn table_name(&self) -> &str {
		"user_link"
	}
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq)]
pub struct Model {
	pub service: String,
	pub user_id: Uuid,
	pub service_userid: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
	Service,
	UserId,
	ServiceUserid,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
	Service,
	UserId,
}

impl PrimaryKeyTrait for PrimaryKey {
	type ValueType = (String, Uuid);
	fn auto_increment() -> bool {
		false
	}
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
	User,
}

impl ColumnTrait for Column {
	type EntityName = Entity;
	fn def(&self) -> ColumnDef {
		match self {
			Self::Service => ColumnType::String(None).def(),
			Self::UserId => ColumnType::Uuid.def(),
			Self::ServiceUserid => ColumnType::String(None).def(),
		}
	}
}

impl RelationTrait for Relation {
	fn def(&self) -> RelationDef {
		match self {
			Self::User => Entity::belongs_to(super::user::Entity)
				.from(Column::UserId)
				.to(super::user::Column::Id)
				.into(),
		}
	}
}

impl Related<super::user::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::User.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
