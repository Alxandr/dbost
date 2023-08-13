use crate::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(User::Table)
					.col(
						ColumnDef::new(User::Id)
							.uuid()
							.primary_key()
							.default(PgFunc::gen_random_uuid())
							.not_null(),
					)
					.col(ColumnDef::new(User::DisplayName).string().not_null())
					.col(ColumnDef::new(User::Email).string().not_null())
					.col(ColumnDef::new(User::AvatarUrl).string().null())
					.index(
						Index::create()
							.name("uq-user-email")
							.col(User::Email)
							.unique(),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(UserLink::Table)
					.col(ColumnDef::new(UserLink::Service).string().not_null())
					.col(ColumnDef::new(UserLink::UserId).uuid().not_null())
					.col(ColumnDef::new(UserLink::ServiceUserId).string().not_null())
					.primary_key(
						// each user can only have one link per service
						Index::create()
							.name("pk-userlink")
							.col(UserLink::Service)
							.col(UserLink::UserId)
							.primary(),
					)
					.index(
						// each service can only have one link per service-user-id
						Index::create()
							.name("uq-userlink_service-userid")
							.col(UserLink::Service)
							.col(UserLink::ServiceUserId)
							.unique()
							.index_type(IndexType::Hash),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-userlink_userid")
							.from(UserLink::Table, UserLink::UserId)
							.to(User::Table, User::Id)
							.on_update(ForeignKeyAction::Cascade)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(Session::Table)
					.col(
						ColumnDef::new(Session::Id)
							.uuid()
							.primary_key()
							.default(PgFunc::gen_random_uuid()),
					)
					.col(ColumnDef::new(Session::CreateTime).timestamp().not_null())
					.col(ColumnDef::new(Session::AccessTime).timestamp().not_null())
					.col(ColumnDef::new(Session::ExpiryTime).timestamp().not_null())
					.col(ColumnDef::new(Session::UserId).uuid().null())
					.foreign_key(
						ForeignKey::create()
							.name("fk-session-user")
							.from(Session::Table, Session::UserId)
							.to(User::Table, User::Id)
							.on_update(ForeignKeyAction::Cascade)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name(Indices::SessionUserId)
					.table(Session::Table)
					.col(Session::UserId)
					.index_type(IndexType::Hash)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name(Indices::SessionExpiryTime)
					.table(Session::Table)
					.col(Session::ExpiryTime)
					.index_type(IndexType::BTree)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_index(Index::drop().name(Indices::SessionExpiryTime).to_owned())
			.await?;

		manager
			.drop_index(Index::drop().name(Indices::SessionUserId).to_owned())
			.await?;

		manager
			.drop_table(Table::drop().table(Session::Table).to_owned())
			.await?;

		manager
			.drop_table(Table::drop().table(UserLink::Table).to_owned())
			.await?;

		manager
			.drop_table(Table::drop().table(User::Table).to_owned())
			.await?;

		Ok(())
	}
}

enum Indices {
	SessionUserId,
	SessionExpiryTime,
}

impl From<Indices> for String {
	fn from(val: Indices) -> Self {
		match val {
			Indices::SessionUserId => "ix-session-userid".into(),
			Indices::SessionExpiryTime => "ix-session-etime".into(),
		}
	}
}
