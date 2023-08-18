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
					.add_column(ColumnDef::new(Series::Description).text().null())
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Season::Table)
					.add_column(ColumnDef::new(Season::Description).text().null())
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
					.drop_column(Series::Description)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Season::Table)
					.drop_column(Season::Description)
					.to_owned(),
			)
			.await?;

		Ok(())
	}
}
