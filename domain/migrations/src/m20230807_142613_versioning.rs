use crate::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(Series::Table)
					.add_column(
						ColumnDef::new(Versioned::Version)
							.timestamp()
							.not_null()
							.default(PgTimeFunc::utc_now()),
					)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Season::Table)
					.add_column(
						ColumnDef::new(Versioned::Version)
							.timestamp()
							.not_null()
							.default(PgTimeFunc::utc_now()),
					)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(ThemeSong::Table)
					.add_column(
						ColumnDef::new(Versioned::Version)
							.timestamp()
							.not_null()
							.default(PgTimeFunc::utc_now()),
					)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(Series::Table)
					.drop_column(Versioned::Version)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Season::Table)
					.drop_column(Versioned::Version)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(ThemeSong::Table)
					.drop_column(Versioned::Version)
					.to_owned(),
			)
			.await?;

		Ok(())
	}
}
