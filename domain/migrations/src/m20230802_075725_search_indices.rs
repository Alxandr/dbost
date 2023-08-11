use crate::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		log_and_exec(manager, "CREATE EXTENSION IF NOT EXISTS pg_trgm;").await?;

		log_and_exec(
			manager,
			format!(
				"CREATE INDEX \"{index}\" ON \"{table}\" USING GIN(\"{col}\" gin_trgm_ops);",
				index = Indices::SeriesNameTrigram.to_string(),
				table = Series::Table.to_string(),
				col = Series::Name.to_string()
			),
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_index(Index::drop().name(Indices::SeriesNameTrigram).to_owned())
			.await?;

		log_and_exec(manager, "DROP EXTENSION IF EXISTS pg_trgm;").await?;

		Ok(())
	}
}

#[derive(Iden)]
enum Indices {
	#[iden = "ix-series_name_trigram"]
	SeriesNameTrigram,
}

impl From<Indices> for String {
	fn from(index: Indices) -> Self {
		index.to_string()
	}
}
