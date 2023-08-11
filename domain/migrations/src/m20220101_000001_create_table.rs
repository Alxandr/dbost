use crate::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(Series::Table)
					.col(ColumnDef::new(Series::Id).uuid().not_null().primary_key())
					.col(ColumnDef::new(Series::Name).string().not_null())
					.col(ColumnDef::new(Series::TvDbId).unsigned().not_null())
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name(Indices::SeriesTvDbId)
					.table(Series::Table)
					.col(Series::TvDbId)
					.unique()
					.to_owned(),
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(Season::Table)
					.col(ColumnDef::new(Season::Id).uuid().not_null().primary_key())
					.col(ColumnDef::new(Season::SeriesId).uuid().not_null())
					.col(ColumnDef::new(Season::Number).small_unsigned().not_null())
					.col(ColumnDef::new(Season::Name).string().null())
					.col(ColumnDef::new(Season::TvDbId).unsigned().not_null())
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name(Indices::SeasonTvDbId)
					.table(Season::Table)
					.col(Season::TvDbId)
					.unique()
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name(Indices::SeasonSeriesId)
					.table(Season::Table)
					.col(Season::SeriesId)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name(Indices::SeasonSeriesIdNumber)
					.table(Season::Table)
					.col(Season::SeriesId)
					.col(Season::Number)
					.to_owned(),
			)
			.await?;

		manager
			.create_foreign_key(
				ForeignKey::create()
					.name(ForeignKeys::SeasonSeriesId)
					.from(Season::Table, Season::SeriesId)
					.to(Series::Table, Series::Id)
					.on_update(ForeignKeyAction::Cascade)
					.on_delete(ForeignKeyAction::Cascade)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_foreign_key(
				ForeignKey::drop()
					.name(ForeignKeys::SeasonSeriesId)
					.to_owned(),
			)
			.await?;

		manager
			.drop_index(Index::drop().name(Indices::SeasonSeriesIdNumber).to_owned())
			.await?;

		manager
			.drop_index(Index::drop().name(Indices::SeasonSeriesId).to_owned())
			.await?;

		manager
			.drop_index(Index::drop().name(Indices::SeasonTvDbId).to_owned())
			.await?;

		manager
			.drop_index(Index::drop().name(Indices::SeriesTvDbId).to_owned())
			.await?;

		manager
			.drop_table(Table::drop().table(Season::Table).to_owned())
			.await?;

		manager
			.drop_table(Table::drop().table(Series::Table).to_owned())
			.await?;

		Ok(())
	}
}

enum Indices {
	SeriesTvDbId,
	SeasonTvDbId,
	SeasonSeriesId,
	SeasonSeriesIdNumber,
}

impl From<Indices> for String {
	fn from(val: Indices) -> Self {
		match val {
			Indices::SeriesTvDbId => "ix-series_tvdbid".to_owned(),
			Indices::SeasonTvDbId => "ix-season_tvdbid".to_owned(),
			Indices::SeasonSeriesId => "ix-season_seriesid".to_owned(),
			Indices::SeasonSeriesIdNumber => "ix-season_seriesid_number".to_owned(),
		}
	}
}

enum ForeignKeys {
	SeasonSeriesId,
}

impl From<ForeignKeys> for String {
	fn from(val: ForeignKeys) -> Self {
		match val {
			ForeignKeys::SeasonSeriesId => "fk-season_seriesid".to_owned(),
		}
	}
}
