mod auth;
mod pagination;

use crate::{extractors::Db, web::pagination::Pagination, AppState};
use axum::{
	extract::{OriginalUri, Query},
	http::StatusCode,
	response::IntoResponse,
	routing::get,
	Router,
};
use dbost_entities::{season, series, user};
use dbost_htmx::extractors::HxRequestInfo;
use dbost_session::Session;
use rstml_component::{write_html, For, HtmlComponent, HtmlContent, HtmlFormatter};
use rstml_component_axum::Html;
use sea_orm::{
	ColumnTrait, EntityTrait, FromQueryResult, PaginatorTrait, QueryOrder, QuerySelect,
	RelationTrait, TransactionError,
};
use sea_query::JoinType;
use serde::Deserialize;
use std::{borrow::Cow, error, fmt, sync::Arc};
use thiserror::Error;
use tracing::log::warn;
use uuid::Uuid;

use self::pagination::PageNumber;

#[derive(Error, Debug)]
enum WebError {
	// #[error("Page not found")]
	// NotFound,
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
			// Self::NotFound => (StatusCode::NOT_FOUND, "Series not found").into_response(),
			Self::DbError(_) => {
				(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
			}

			Self::TvDbError(_) => {
				(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
			}
		}
	}
}

#[derive(HtmlComponent)]
struct NavSearchBox;

impl HtmlContent for NavSearchBox {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		write_html!(formatter,
			<div class="form-control">
				<input type="text" placeholder="Search" class="w-24 input input-bordered md:w-auto" />
			</div>
		)
	}
}

#[derive(HtmlComponent)]
struct UserDropdown<'a> {
	user: Option<&'a user::Model>,
}

impl<'a> HtmlContent for UserDropdown<'a> {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		match self.user {
			None => write_html!(formatter,
				<div>
					<label tabindex="0" class="btn btn-ghost btn-circle avatar">
						<a href="/auth/login/github" class="w-10 rounded-full">
							<svg width="20" height="20" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" class="inline-block w-5 h-5 fill-current md:h-6 md:w-6">
								<path d="M256,32C132.3,32,32,134.9,32,261.7c0,101.5,64.2,187.5,153.2,217.9a17.56,17.56,0,0,0,3.8.4c8.3,0,11.5-6.1,11.5-11.4,0-5.5-.2-19.9-.3-39.1a102.4,102.4,0,0,1-22.6,2.7c-43.1,0-52.9-33.5-52.9-33.5-10.2-26.5-24.9-33.6-24.9-33.6-19.5-13.7-.1-14.1,1.4-14.1h.1c22.5,2,34.3,23.8,34.3,23.8,11.2,19.6,26.2,25.1,39.6,25.1a63,63,0,0,0,25.6-6c2-14.8,7.8-24.9,14.2-30.7-49.7-5.8-102-25.5-102-113.5,0-25.1,8.7-45.6,23-61.6-2.3-5.8-10-29.2,2.2-60.8a18.64,18.64,0,0,1,5-.5c8.1,0,26.4,3.1,56.6,24.1a208.21,208.21,0,0,1,112.2,0c30.2-21,48.5-24.1,56.6-24.1a18.64,18.64,0,0,1,5,.5c12.2,31.6,4.5,55,2.2,60.8,14.3,16.1,23,36.6,23,61.6,0,88.2-52.4,107.6-102.3,113.3,8,7.1,15.2,21.1,15.2,42.5,0,30.7-.3,55.5-.3,63,0,5.4,3.1,11.5,11.4,11.5a19.35,19.35,0,0,0,4-.4C415.9,449.2,480,363.1,480,261.7,480,134.9,379.7,32,256,32Z" />
							</svg>
						</a>
					</label>
				</div>
			),
			Some(user) => {
				let avatar_url = user
					.avatar_url
					.as_deref()
					.map(Cow::Borrowed)
					.unwrap_or_else(|| {
						Cow::Owned(format!(
							"https://www.gravatar.com/avatar/{:x}?d=mp",
							md5::compute(user.email.as_bytes())
						))
					});
				write_html!(formatter,
					<div class="dropdown dropdown-end" id="navbar-user">
						<label tabindex="0" class="btn btn-ghost btn-circle avatar">
							<div class="w-10 rounded-full">
								<img src=&*avatar_url referrerpolicy="no-referrer" />
							</div>
						</label>

						<ul tabindex="0" class="mt-3 z-[1] p-2 shadow menu menu-sm dropdown-content bg-base-100 rounded-box w-52">
							<li><a>"Profile"</a></li>
							<li><a href="/auth/logout">"Logout"</a></li>
						</ul>
					</div>
				)
			}
		}
	}
}

#[derive(HtmlComponent)]
struct NavBar<'a> {
	user: Option<&'a user::Model>,
}

impl<'a> HtmlContent for NavBar<'a> {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		write_html!(formatter,
			<nav class="navbar bg-base-100">
				// <div class="flex-none">
				// 	<button class="btn btn-square btn-ghost">
				// 		<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="inline-block w-5 h-5 stroke-current">
				// 			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
				// 		</svg>
				// 	</button>
				// </div>

				<div class="flex-1">
					<a class="text-xl normal-case btn btn-ghost hover:bg-transparent">"dBost"</a>
					<span class="normal-case text-normal">"| ˈdi: buːst |"</span>
				</div>

				// search
				<div class="flex-none gap-2">
					<NavSearchBox />
					<UserDropdown user=self.user />
				</div>
			</nav>
		)
	}
}

#[derive(HtmlComponent)]
struct Template<T, C>
where
	T: AsRef<str>,
	C: HtmlContent,
{
	pub title: T,
	pub children: C,
	pub session: Session,
}

impl<T, C> HtmlContent for Template<T, C>
where
	T: AsRef<str>,
	C: HtmlContent,
{
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		write_html!(formatter,
			<!DOCTYPE html>
			<html>
				<head>
					<meta charset="UTF-8" />
					<link rel="apple-touch-icon" sizes="57x57" href="/apple-icon-57x57.png">
					<link rel="apple-touch-icon" sizes="60x60" href="/apple-icon-60x60.png">
					<link rel="apple-touch-icon" sizes="72x72" href="/apple-icon-72x72.png">
					<link rel="apple-touch-icon" sizes="76x76" href="/apple-icon-76x76.png">
					<link rel="apple-touch-icon" sizes="114x114" href="/apple-icon-114x114.png">
					<link rel="apple-touch-icon" sizes="120x120" href="/apple-icon-120x120.png">
					<link rel="apple-touch-icon" sizes="144x144" href="/apple-icon-144x144.png">
					<link rel="apple-touch-icon" sizes="152x152" href="/apple-icon-152x152.png">
					<link rel="apple-touch-icon" sizes="180x180" href="/apple-icon-180x180.png">
					<link rel="icon" type="image/png" sizes="192x192"  href="/android-icon-192x192.png">
					<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png">
					<link rel="icon" type="image/png" sizes="96x96" href="/favicon-96x96.png">
					<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png">
					<link rel="icon" type="image/x-icon" href="/favicon.ico">
					<link rel="manifest" href="/manifest.json">
					<meta name="msapplication-TileColor" content="#1d232a">
					<meta name="msapplication-TileImage" content="/ms-icon-144x144.png">
					<meta name="theme-color" content="#1d232a">
					<title>{self.title.as_ref()}</title>
					<link rel="stylesheet" type="text/css" href="/main.css" />
				</head>
				<body>
					<NavBar user=self.session.user().as_deref() />
					<main class="p-8">
						{self.children}
					</main>
				</body>
			</html>
		)
	}
}

#[derive(FromQueryResult)]
struct SeriesCardDb {
	name: String,
	id: Uuid,
	image: Option<String>,
	season_count: i64,
}

#[derive(HtmlComponent)]
struct SeriesCard {
	name: String,
	id: Uuid,
	image: Option<String>,
	season_count: i64,
	next_page_link: Option<Arc<str>>,
}

impl SeriesCard {
	pub fn new(db: SeriesCardDb, next_page_link: Option<Arc<str>>) -> Self {
		Self {
			name: db.name,
			id: db.id,
			image: db.image,
			season_count: db.season_count,
			next_page_link,
		}
	}
}

impl HtmlContent for SeriesCard {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		write_html!(formatter,
			<li id=("series-card-", self.id.to_string()) class="grid grid-cols-1 row-span-2 gap-0 overflow-hidden shadow-xl rounded-box bg-base-100 grid-rows-subgrid">
				<figure style="grid-row: 1 / span 2; grid-column: 1 / 1;">
					<img src=self.image.as_deref() alt=(&*self.name, " image") />
				</figure>
				<div class="p-4 text-base bg-base-100/80" style="grid-row: 2 / span 1; grid-column: 1 / 1;">
					<h2 class="card-title">{self.name}</h2>
					<p>"Seasons: " {self.season_count}</p>
				</div>
			</li>
		)
	}
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
	HxRequestInfo(_): HxRequestInfo,
) -> Result<impl IntoResponse, WebError> {
	let paginator = series::Entity::find()
		.select_only()
		.column(series::Column::Name)
		.column(series::Column::Id)
		.column(series::Column::Image)
		.column_as(season::Column::Id.count(), "season_count")
		.join(JoinType::LeftJoin, series::Relation::Season.def())
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

		SeriesCard::new(s, next_page_link)
	});

	let html = Html::from_fn(move |f| {
		write_html!(f,
			<Template title="Series" session=session>
				<h1 class="mb-8 text-4xl font-bold">Series</h1>

				<ul class="grid grid-cols-1 gap-4 auto-rows-cards sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
					<For items={series}>
						{ move |f, s| s.fmt(f) }
					</For>
				</ul>

				{pagination}
			</Template>
		)
	});

	Ok(html.into_response())
}

pub fn router() -> Router<AppState> {
	Router::new()
		.nest("/auth", auth::router())
		.route("/", get(index))
}
