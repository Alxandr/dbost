use crate::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const SERIES_NAME_INDEX: &str = "ix-series-name";
const SEASON_SERIESID: &str = "ix-season-seriesid";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_index(
				Index::create()
					.name(SERIES_NAME_INDEX)
					.table(Series::Table)
					.col(Series::Name)
					.index_type(IndexType::BTree)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name(SEASON_SERIESID)
					.table(Season::Table)
					.col(Season::SeriesId)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_index(Index::drop().name(SERIES_NAME_INDEX).to_owned())
			.await?;

		manager
			.drop_index(Index::drop().name(SEASON_SERIESID).to_owned())
			.await?;

		Ok(())
	}
}
