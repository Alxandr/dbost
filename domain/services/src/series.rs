use crate::macros::define_service;
use dbost_entities::{season, series};
use dbost_utils::ActiveValueExt;
use futures::{future::BoxFuture, FutureExt};
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait,
	QueryFilter, TransactionError, TransactionTrait, TryIntoModel,
};
use std::{collections::BTreeMap, sync::Arc};
use thiserror::Error;
use tvdb_client::TvDbClient;
use uuid::Uuid;

define_service! {
	#[derive(Clone)]
	pub struct SeriesService {
		pub db: DatabaseConnection,
		pub tvdb: Arc<TvDbClient>,
	}
}

#[derive(Debug, Clone, Copy)]
pub enum SeriesRef {
	Id(Uuid),
	TvDbId(u64),
}

impl From<Uuid> for SeriesRef {
	fn from(value: Uuid) -> Self {
		Self::Id(value)
	}
}

#[derive(Debug, Error)]
pub enum SeriesServiceError {
	#[error("series not found: {0:?}")]
	NotFound(SeriesRef),

	#[error(transparent)]
	DbErr(#[from] DbErr),

	#[error(transparent)]
	TvDbError(#[from] tvdb_client::TvDbError),
}

impl From<TransactionError<SeriesServiceError>> for SeriesServiceError {
	fn from(value: TransactionError<SeriesServiceError>) -> Self {
		match value {
			TransactionError::Connection(db) => db.into(),
			TransactionError::Transaction(inner) => inner,
		}
	}
}

pub struct SeriesWithSeasons {
	pub series: series::Model,
	pub seasons: Vec<season::Model>,
}

impl SeriesWithSeasons {
	pub fn new(series: series::Model, seasons: Vec<season::Model>) -> Self {
		Self { series, seasons }
	}
}

impl SeriesService {
	pub async fn get_series(
		&self,
		id: impl Into<SeriesRef>,
	) -> Result<Option<SeriesWithSeasons>, SeriesServiceError> {
		async fn get_series(
			service: &SeriesService,
			id: SeriesRef,
		) -> Result<Option<SeriesWithSeasons>, SeriesServiceError> {
			let series = match id {
				SeriesRef::Id(v) => series::Entity::find_by_id(v).one(&service.db).await?,
				SeriesRef::TvDbId(v) => {
					series::Entity::find()
						.filter(series::Column::TvdbId.eq(v as i32))
						.one(&service.db)
						.await?
				}
			};

			let series = match series {
				None => return Ok(None),
				Some(series) => series,
			};

			let seasons = season::Entity::find()
				.filter(season::Column::SeriesId.eq(series.id))
				.all(&service.db)
				.await?;

			Ok(Some(SeriesWithSeasons::new(series, seasons)))
		}

		get_series(self, id.into()).await
	}

	pub async fn fetch_from_tvdb(
		&self,
		id: u64,
		transaction: Option<&DatabaseTransaction>,
	) -> Result<Option<SeriesWithSeasons>, SeriesServiceError> {
		async fn insert_seasons_db(
			tx: &DatabaseTransaction,
			series_id: Uuid,
			seasons: impl IntoIterator<Item = tvdb_client::Season>,
		) -> Result<Vec<season::Model>, SeriesServiceError> {
			use sea_orm::ActiveValue::*;

			let (ids, seasons): (Vec<_>, Vec<_>) = seasons
				.into_iter()
				.map(|update| {
					let season_id = Uuid::new_v4();
					(
						season_id,
						season::ActiveModel {
							id: Set(season_id),
							name: Set(update.name),
							description: Set(update.description),
							number: Set(update.number as i16),
							tvdb_id: Set(update.id as i32),
							series_id: Set(series_id),
							image: Set(update.image),
							theme_song_id: Set(None),
							version: NotSet,
						},
					)
				})
				.unzip();

			season::Entity::insert_many(seasons)
				.on_empty_do_nothing()
				.exec(tx)
				.await?;

			let seasons = season::Entity::find()
				.filter(season::Column::Id.is_in(ids))
				.all(tx)
				.await?;

			Ok(seasons)
		}

		async fn insert_series_db(
			tx: &DatabaseTransaction,
			update: tvdb_client::Series,
		) -> Result<SeriesWithSeasons, SeriesServiceError> {
			use sea_orm::ActiveValue::*;

			let series = series::ActiveModel {
				id: Set(Uuid::new_v4()),
				name: Set(update.name),
				description: Set(update.description),
				tvdb_id: Set(update.id as i32),
				image: Set(update.image),
				theme_song_id: Set(None),
				version: NotSet,
			};

			let series = series.insert(tx).await?;

			let seasons = insert_seasons_db(tx, series.id, update.seasons).await?;

			Ok(SeriesWithSeasons::new(series, seasons))
		}

		async fn update_series_db(
			tx: &DatabaseTransaction,
			update: tvdb_client::Series,
			series: series::Model,
			seasons: Vec<season::Model>,
		) -> Result<SeriesWithSeasons, SeriesServiceError> {
			let mut series: series::ActiveModel = series.into();
			series.name.update(update.name);
			series.description.update(update.description);
			if let Some(image) = update.image {
				series.image.update(Some(image));
			}

			let series = if series.is_changed() {
				series.update(tx).await?
			} else {
				series.try_into_model()?
			};

			let old_seasons = seasons;
			let mut seasons = Vec::with_capacity(update.seasons.len());
			let mut to_delete = Vec::with_capacity(old_seasons.len());

			let mut updates = update
				.seasons
				.into_iter()
				.map(|s| (s.id as i32, s))
				.collect::<BTreeMap<_, _>>();

			for season in old_seasons {
				match updates.remove(&season.tvdb_id) {
					Some(update) => {
						let mut season: season::ActiveModel = season.into();
						season.name.update(update.name);
						season.description.update(update.description);
						season.number.update(update.number as i16);
						if let Some(image) = update.image {
							season.image.update(Some(image));
						}

						let season = if season.is_changed() {
							season.update(tx).await?
						} else {
							season.try_into_model()?
						};
						seasons.push(season);
					}
					None => {
						to_delete.push(season);
					}
				}
			}

			if !updates.is_empty() {
				seasons.extend(insert_seasons_db(tx, series.id, updates.into_values()).await?);
			}

			if !to_delete.is_empty() {
				season::Entity::delete_many()
					.filter(season::Column::Id.is_in(to_delete.into_iter().map(|s| s.id)))
					.exec(tx)
					.await?;
			}

			Ok(SeriesWithSeasons::new(series, seasons))
		}

		async fn insert_or_update_series_db(
			tx: &DatabaseTransaction,
			update: tvdb_client::Series,
			series: Option<series::Model>,
			seasons: Vec<season::Model>,
		) -> Result<SeriesWithSeasons, SeriesServiceError> {
			match series {
				None => {
					debug_assert!(seasons.is_empty());
					insert_series_db(tx, update).await
				}
				Some(series) => update_series_db(tx, update, series, seasons).await,
			}
		}

		let tx = match transaction {
			None => {
				fn run_in_transaction(
					service: SeriesService,
					id: u64,
					transaction: &DatabaseTransaction,
				) -> BoxFuture<'_, Result<Option<SeriesWithSeasons>, SeriesServiceError>> {
					async move { service.fetch_from_tvdb(id, Some(transaction)).await }.boxed()
				}

				let self_clone = self.clone();
				return self
					.db
					.transaction(move |tx| run_in_transaction(self_clone, id, tx))
					.await
					.map_err(SeriesServiceError::from);
			}
			Some(tx) => tx,
		};

		let from_tvdb = self.tvdb.get_series(id).await?;

		let from_tvdb = match from_tvdb {
			None => return Ok(None),
			Some(v) => v,
		};

		let series = series::Entity::find()
			.filter(series::Column::TvdbId.eq(id as i32))
			.one(tx)
			.await?;

		let seasons = match series.as_ref() {
			None => Vec::new(),
			Some(e) => {
				season::Entity::find()
					.filter(season::Column::SeriesId.eq(e.id))
					.all(tx)
					.await?
			}
		};

		insert_or_update_series_db(tx, from_tvdb, series, seasons)
			.await
			.map(Some)
	}
}
