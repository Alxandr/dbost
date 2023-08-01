use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(Series::Table)
					.modify_column(ColumnDef::new(Series::Id).default(PgFunc::gen_random_uuid()))
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Season::Table)
					.modify_column(ColumnDef::new(Season::Id).default(PgFunc::gen_random_uuid()))
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(ThemeSong::Table)
					.modify_column(ColumnDef::new(ThemeSong::Id).default(PgFunc::gen_random_uuid()))
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
		// this doesn't really have to change anything, cause we've just added a
		// default value to a couple of columns, which if left doesn't cause any
		// issues.
		Ok(())
	}
}

#[derive(Iden)]
enum ThemeSong {
	Table,
	Id,
}

#[derive(Iden)]
enum Series {
	Table,
	Id,
}

#[derive(Iden)]
enum Season {
	Table,
	Id,
}
