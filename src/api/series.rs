use crate::{
	extractors::{Db, TvDb},
	AppState,
};
use axum::{
	extract::{Path, Query},
	http::StatusCode,
	response::IntoResponse,
	routing::get,
	Json, Router,
};
use futures::FutureExt;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, TransactionError,
	TransactionTrait, TryIntoModel,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error};
use theme_db_entities::{prelude::*, season, series};
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

trait ResultExt<T, E> {
	fn log_err(self, f: impl FnOnce(&E)) -> Self;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
	fn log_err(self, f: impl FnOnce(&E)) -> Self {
		if let Err(e) = &self {
			f(e);
		}

		self
	}
}

trait ActiveValueExt<T> {
	fn update(&mut self, value: T);
}

impl<T> ActiveValueExt<T> for sea_orm::ActiveValue<T>
where
	T: Into<sea_orm::Value>,
	for<'a> &'a T: Eq,
{
	fn update(&mut self, value: T) {
		match self {
			Self::Set(v) => *v = value,
			Self::NotSet => *self = Self::Set(value),
			Self::Unchanged(v) => {
				if &*v != &value {
					*self = Self::Set(value);
				}
			}
		}
	}
}

#[derive(Error, Debug)]
enum SeriesError {
	#[error("Series not found")]
	NotFound,

	#[error("Database error: {0}")]
	DbError(#[from] sea_orm::error::DbErr),

	#[error("TvDb client error: {0}")]
	TvDbError(#[from] tvdb_client::TvDbError),
}

impl<E> From<TransactionError<E>> for SeriesError
where
	E: Into<SeriesError> + error::Error,
{
	fn from(value: TransactionError<E>) -> Self {
		match value {
			TransactionError::Connection(e) => e.into(),
			TransactionError::Transaction(e) => e.into(),
		}
	}
}

impl IntoResponse for SeriesError {
	fn into_response(self) -> axum::response::Response {
		match self {
			Self::NotFound => (StatusCode::NOT_FOUND, "Series not found").into_response(),

			Self::DbError(_) => {
				(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
			}

			Self::TvDbError(_) => {
				(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
			}
		}
	}
}

async fn get_series(Path(id): Path<Uuid>, Db(db): Db) -> Result<impl IntoResponse, SeriesError> {
	let series = Series::find_by_id(id)
		.one(&db)
		.await
		.log_err(|e| {
			error!("Error finding series: {}", e);
		})?
		.ok_or(SeriesError::NotFound)?;

	let seasons = Season::find()
		.filter(season::Column::SeriesId.eq(series.id))
		.all(&db)
		.await
		.log_err(|e| {
			error!("Error finding seasons: {}", e);
		})?;

	Ok(Json(SeriesDto::new(series, seasons)))
}

#[derive(Deserialize)]
struct GetSeriesQuery {
	#[serde(default = "Default::default")]
	update: bool,
}

async fn get_series_by_tvdb_id(
	Path(id): Path<u32>,
	Query(query): Query<GetSeriesQuery>,
	Db(db): Db,
	TvDb(tvdb): TvDb,
) -> Result<impl IntoResponse, SeriesError> {
	let from_tvdb = if query.update {
		tvdb.get_series(id as u64).await.log_err(|e| {
			error!("Error fetching series from TVDB: {}", e);
		})?
	} else {
		None
	};

	let mut series = Series::find()
		.filter(series::Column::TvdbId.eq(id as i32))
		.one(&db)
		.await
		.log_err(|e| {
			error!("Error finding series: {}", e);
		})?;

	let mut seasons = match series.as_ref() {
		None => Vec::new(),
		Some(e) => Season::find()
			.filter(season::Column::SeriesId.eq(e.id))
			.all(&db)
			.await
			.log_err(|e| {
				error!("Error finding seasons: {}", e);
			})?,
	};

	if let Some(update) = from_tvdb {
		let result = db
			.transaction(|tx| insert_or_update_series_db(tx, update, series, seasons).boxed())
			.await?;
		series = Some(result.0);
		seasons = result.1;
	}

	series
		.map(|series| SeriesDto::new(series, seasons))
		.map(Json)
		.ok_or(SeriesError::NotFound)
}

async fn insert_seasons_db(
	tx: &DatabaseTransaction,
	series_id: Uuid,
	seasons: impl IntoIterator<Item = tvdb_client::Season>,
) -> Result<Vec<season::Model>, SeriesError> {
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
					number: Set(update.number as i16),
					tvdb_id: Set(update.id as i32),
					series_id: Set(series_id),
				},
			)
		})
		.unzip();

	Season::insert_many(seasons)
		.on_empty_do_nothing()
		.exec(tx)
		.await
		.log_err(|e| {
			error!("Error inserting seasons: {}", e);
		})?;

	let seasons = Season::find()
		.filter(season::Column::Id.is_in(ids))
		.all(tx)
		.await
		.log_err(|e| {
			error!("Error finding seasons: {}", e);
		})?;

	Ok(seasons)
}

async fn insert_series_db(
	tx: &DatabaseTransaction,
	update: tvdb_client::Series,
) -> Result<(series::Model, Vec<season::Model>), SeriesError> {
	use sea_orm::ActiveValue::*;

	let series = series::ActiveModel {
		id: Set(Uuid::new_v4()),
		name: Set(update.name),
		tvdb_id: Set(update.id as i32),
	};

	let series = series.insert(tx).await.log_err(|e| {
		error!("Error inserting series: {}", e);
	})?;

	let seasons = insert_seasons_db(tx, series.id, update.seasons).await?;

	Ok((series, seasons))
}

async fn update_series_db(
	tx: &DatabaseTransaction,
	update: tvdb_client::Series,
	series: series::Model,
	seasons: Vec<season::Model>,
) -> Result<(series::Model, Vec<season::Model>), SeriesError> {
	let mut series: series::ActiveModel = series.into();
	series.name.update(update.name);

	let series = if series.is_changed() {
		series.update(tx).await.log_err(|e| {
			error!("Error updating series: {}", e);
		})?
	} else {
		series.try_into_model().log_err(|e| {
			error!("Error updating series: {}", e);
		})?
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
				season.number.update(update.number as i16);

				let season = if season.is_changed() {
					season.update(tx).await.log_err(|e| {
						error!("Error updating season: {}", e);
					})?
				} else {
					season.try_into_model().log_err(|e| {
						error!("Error updating season: {}", e);
					})?
				};
				seasons.push(season);
			}
			None => {
				to_delete.push(season);
			}
		}
	}

	seasons.extend(insert_seasons_db(tx, series.id, updates.into_values()).await?);

	if !to_delete.is_empty() {
		Season::delete_many()
			.filter(season::Column::Id.is_in(to_delete.into_iter().map(|s| s.id)))
			.exec(tx)
			.await
			.log_err(|e| {
				error!("Error deleting seasons: {}", e);
			})?;
	}

	Ok((series, seasons))
}

async fn insert_or_update_series_db(
	tx: &DatabaseTransaction,
	update: tvdb_client::Series,
	series: Option<series::Model>,
	seasons: Vec<season::Model>,
) -> Result<(series::Model, Vec<season::Model>), SeriesError> {
	match series {
		None => {
			debug_assert!(seasons.is_empty());
			insert_series_db(tx, update).await
		}
		Some(series) => update_series_db(tx, update, series, seasons).await,
	}
}

pub fn router() -> Router<AppState> {
	Router::new()
		.route("/:id", get(get_series))
		.route("/tvdb/:id", get(get_series_by_tvdb_id))
}

#[derive(Serialize)]
struct SeriesDto {
	pub id: Uuid,
	pub name: String,
	pub tvdb_id: u32,
	pub seasons: Vec<SeasonDto>,
}

#[derive(Serialize)]
struct SeasonDto {
	pub id: Uuid,
	pub number: i32,
	pub name: Option<String>,
	pub tvdb_id: u32,
}

impl SeriesDto {
	fn new(series: series::Model, seasons: Vec<season::Model>) -> Self {
		Self {
			id: series.id,
			name: series.name,
			tvdb_id: series.tvdb_id as u32,
			seasons: seasons.into_iter().map(SeasonDto::new).collect(),
		}
	}
}

impl SeasonDto {
	fn new(season: season::Model) -> Self {
		Self {
			id: season.id,
			number: season.number as i32,
			name: season.name,
			tvdb_id: season.tvdb_id as u32,
		}
	}
}
