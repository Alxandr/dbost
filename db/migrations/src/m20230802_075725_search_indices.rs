use sea_orm_migration::{prelude::*, sea_orm::ExecResult};

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn log_and_exec<'a, 'b>(
	manager: &'a SchemaManager<'b>,
	sql: impl Into<String>,
) -> Result<ExecResult, DbErr> {
	let sql = sql.into();
	println!("Executing SQL: {}", sql);
	manager.get_connection().execute_unprepared(&sql).await
}

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

#[derive(Iden)]
enum Series {
	Table,
	Name,
}
