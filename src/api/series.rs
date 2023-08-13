use crate::AppState;
use axum::{
	extract::{FromRequestParts, Path, Query},
	http::StatusCode,
	response::IntoResponse,
	routing::get,
	Json, Router,
};
use dbost_entities::{season, series};
use dbost_services::series::{SeriesRef, SeriesService};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

static_assertions::assert_impl_all!(SeriesService: FromRequestParts<AppState>);

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

async fn get_series(Path(id): Path<Uuid>, service: SeriesService) -> impl IntoResponse {
	let series = match service.get_series(id).await {
		Ok(Some(series)) => series,
		Ok(None) => return (StatusCode::NOT_FOUND, "Series not found").into_response(),
		Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response(),
	};

	Json(SeriesDto::new(series.series, series.seasons)).into_response()
}

#[derive(Deserialize)]
struct GetSeriesQuery {
	#[serde(default = "Default::default")]
	update: bool,
}

async fn get_series_by_tvdb_id(
	Path(id): Path<u64>,
	Query(query): Query<GetSeriesQuery>,
	service: SeriesService,
) -> impl IntoResponse {
	let lookup = if query.update {
		service.fetch_from_tvdb(id, None).await
	} else {
		service.get_series(SeriesRef::TvDbId(id)).await
	};

	let series = match lookup {
		Ok(Some(series)) => series,
		Ok(None) => return (StatusCode::NOT_FOUND, "Series not found").into_response(),
		Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response(),
	};

	Json(SeriesDto::new(series.series, series.seasons)).into_response()
}

pub fn router() -> Router<AppState> {
	Router::<AppState>::new()
		.route("/:id", get(get_series))
		.route("/tvdb/:id", get(get_series_by_tvdb_id))
}

#[derive(Serialize)]
struct SeriesDto {
	pub id: Uuid,
	pub name: String,
	pub tvdb_id: u32,
	pub seasons: Vec<SeasonDto>,
	pub image: Option<String>,
}

#[derive(Serialize)]
struct SeasonDto {
	pub id: Uuid,
	pub number: i32,
	pub name: Option<String>,
	pub tvdb_id: u32,
	pub image: Option<String>,
}

impl SeriesDto {
	fn new(series: series::Model, seasons: Vec<season::Model>) -> Self {
		Self {
			id: series.id,
			name: series.name,
			tvdb_id: series.tvdb_id as u32,
			seasons: seasons.into_iter().map(SeasonDto::new).collect(),
			image: series.image,
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
			image: season.image,
		}
	}
}
