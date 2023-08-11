use crate::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(ThemeSong::Table)
					.col(
						ColumnDef::new(ThemeSong::Id)
							.uuid()
							.not_null()
							.primary_key(),
					)
					.col(ColumnDef::new(ThemeSong::Name).string().not_null())
					.col(ColumnDef::new(ThemeSong::YouTubeId).string().null())
					.col(ColumnDef::new(ThemeSong::YouTubeStartsAt).unsigned().null())
					.col(ColumnDef::new(ThemeSong::YouTubeEndsAt).unsigned().null())
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Series::Table)
					.add_column(
						ColumnDef::new(Series::ThemeSongId)
							.uuid()
							.null()
							.default(String::null()),
					)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Season::Table)
					.add_column(
						ColumnDef::new(Season::ThemeSongId)
							.uuid()
							.null()
							.default(String::null()),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_foreign_key(
				ForeignKey::create()
					.name(ForeignKeys::SeriesThemeSongId)
					.from(Series::Table, Series::ThemeSongId)
					.to(ThemeSong::Table, ThemeSong::Id)
					.on_update(ForeignKeyAction::Cascade)
					.on_delete(ForeignKeyAction::SetNull)
					.to_owned(),
			)
			.await?;

		manager
			.create_foreign_key(
				ForeignKey::create()
					.name(ForeignKeys::SeasonThemeSongId)
					.from(Season::Table, Season::ThemeSongId)
					.to(ThemeSong::Table, ThemeSong::Id)
					.on_update(ForeignKeyAction::Cascade)
					.on_delete(ForeignKeyAction::SetNull)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_foreign_key(
				ForeignKey::drop()
					.name(ForeignKeys::SeriesThemeSongId)
					.to_owned(),
			)
			.await?;

		manager
			.drop_foreign_key(
				ForeignKey::drop()
					.name(ForeignKeys::SeasonThemeSongId)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Series::Table)
					.drop_column(Series::ThemeSongId)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Season::Table)
					.drop_column(Season::ThemeSongId)
					.to_owned(),
			)
			.await?;

		manager
			.drop_table(Table::drop().table(ThemeSong::Table).to_owned())
			.await?;

		Ok(())
	}
}

enum ForeignKeys {
	SeriesThemeSongId,
	SeasonThemeSongId,
}

impl From<ForeignKeys> for String {
	fn from(val: ForeignKeys) -> Self {
		match val {
			ForeignKeys::SeriesThemeSongId => "fk-series_themesong".to_owned(),
			ForeignKeys::SeasonThemeSongId => "fk-season_themesong".to_owned(),
		}
	}
}
