mod auth;
mod pagination;
mod views;

use self::{
	pagination::PageNumber,
	views::{IndexPage, SeriesCard, SeriesEdit, SeriesPage},
};
use crate::{extractors::Db, utils::Concat, web::pagination::Pagination, AppState};
use axum::{
	body::BoxBody,
	extract::{OriginalUri, Path, Query},
	http::{Response, StatusCode},
	response::{IntoResponse, Redirect},
	routing::get,
	Router,
};
use dbost_entities::{season, series, theme_song};
use dbost_htmx::extractors::{HtmxRequestInfo, HxRequestInfo};
use dbost_session::Session;
use indexmap::IndexMap;
use sea_orm::{
	ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
	QueryOrder, QuerySelect, RelationTrait, TransactionError,
};
use sea_query::JoinType;
use serde::Deserialize;
use std::{error, sync::Arc};
use thiserror::Error;
use tracing::log::warn;
use uuid::Uuid;

#[derive(Error, Debug)]
enum WebError {
	#[error("Page not found")]
	NotFound,

	#[error("Database error: {0}")]
	DbError(#[from] sea_orm::error::DbErr),

	#[error("TvDb client error: {0}")]
	TvDbError(#[from] tvdb_client::TvDbError),
}

impl<E> From<TransactionError<E>> for WebError
where
	E: Into<WebError> + error::Error,
{
	fn from(value: TransactionError<E>) -> Self {
		match value {
			TransactionError::Connection(e) => e.into(),
			TransactionError::Transaction(e) => e.into(),
		}
	}
}

impl IntoResponse for WebError {
	fn into_response(self) -> axum::response::Response {
		warn!("{}", self);
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

#[derive(FromQueryResult)]
struct SeriesCardDb {
	name: String,
	id: Uuid,
	image: Option<String>,
	season_count: i64,
}

#[derive(Deserialize)]
struct CallbackQuery {
	// #[serde(default)]
	page: PageNumber,
}

async fn index(
	Db(db): Db,
	session: Session,
	Query(query): Query<CallbackQuery>,
	OriginalUri(uri): OriginalUri,
	HxRequestInfo(hx): HxRequestInfo,
) -> Result<Response<BoxBody>, WebError> {
	let paginator = series::Entity::find()
		.select_only()
		.column(series::Column::Name)
		.column(series::Column::Id)
		.column(series::Column::Image)
		.column_as(season::Column::Id.count(), "season_count")
		.join(JoinType::LeftJoin, series::Relation::Season.def())
		.filter(season::Column::Number.ne(0))
		.group_by(series::Column::Id)
		.order_by_asc(series::Column::Name)
		.into_model::<SeriesCardDb>()
		.paginate(&db, 60);

	let pages = paginator.num_pages().await?;
	if pages > 0 && query.page >= pages {
		return Ok((StatusCode::NOT_FOUND, "Page not found").into_response());
	}

	let pagination = Pagination::new(pages, query.page, uri);

	let next_page_link: Option<Arc<str>> = pagination.next_page_href().map(Arc::from);
	let series = paginator.fetch_page(query.page.index()).await?;
	let series_count = series.len();
	let series = series.into_iter().enumerate().map(|(i, s)| {
		let next_page_link = match i == series_count - 1 {
			true => next_page_link.clone(),
			false => None,
		};

		SeriesCard::new(s.name, s.id, s.image, s.season_count, next_page_link)
	});

	let index = IndexPage::new(&session, series);

	match hx {
		Some(hx) if !hx.boosted => Ok(index.into_items_fragment_response()),
		_ => Ok(index.into_response()),
	}
}

async fn series_view(
	series_id: Uuid,
	db: DatabaseConnection,
	session: Session,
	_: Option<HtmxRequestInfo>,
	edit: SeriesEdit,
) -> Result<Response<BoxBody>, WebError> {
	let series = series::Entity::find_by_id(series_id)
		.one(&db)
		.await?
		.ok_or(WebError::NotFound)?;

	let seasons = season::Entity::find()
		.filter(season::Column::SeriesId.eq(series_id))
		.order_by_asc(season::Column::Number)
		.all(&db)
		.await?;

	let theme_ids = Concat::new(
		series.theme_song_id.into_iter(),
		seasons.iter().filter_map(|s| s.theme_song_id),
	);

	let themes = theme_song::Entity::find()
		.filter(theme_song::Column::Id.is_in(theme_ids))
		.all(&db)
		.await?
		.into_iter()
		.map(|m| (m.id, m))
		.collect::<IndexMap<_, _>>();

	Ok(SeriesPage::new(&session, series, seasons, themes, edit).into_response())
}

async fn series(
	Path(series_id): Path<Uuid>,
	Db(db): Db,
	session: Session,
	HxRequestInfo(hx): HxRequestInfo,
) -> Result<Response<BoxBody>, WebError> {
	series_view(series_id, db, session, hx, SeriesEdit::None).await
}

async fn series_edit_series(
	Path(series_id): Path<Uuid>,
	Db(db): Db,
	session: Session,
	HxRequestInfo(hx): HxRequestInfo,
) -> Result<Response<BoxBody>, WebError> {
	if session.user().is_none() {
		return Ok(Redirect::to(&format!("/series/{series_id}")).into_response());
	}

	series_view(series_id, db, session, hx, SeriesEdit::Series).await
}

pub fn router() -> Router<AppState> {
	Router::new()
		.nest("/auth", auth::router())
		.route("/", get(index))
		.route("/series/:id", get(series))
		.route("/series/:id/edit", get(series_edit_series))
}
