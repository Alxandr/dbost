use crate::{extractors::Db, AppState};
use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};
use dbost_entities::{prelude::*, series};
use rstml_component::{write_html, For, HtmlComponent, HtmlContent, HtmlFormatter};
use rstml_component_axum::Html;
use sea_orm::{EntityTrait, TransactionError};
use std::{error, fmt};
use thiserror::Error;

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

#[derive(HtmlComponent)]
struct NavBar;

impl HtmlContent for NavBar {
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
				<input type="text" placeholder="Search" class="w-24 input input-bordered md:w-auto" />
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
					<title>{self.title.as_ref()}</title>
					<link rel="stylesheet" type="text/css" href="/public/main.css" />
				</head>
				<body>
					<NavBar />
					<main>
						{self.children}
					</main>
				</body>
			</html>
		)
	}
}

#[derive(HtmlComponent)]
struct SeriesCard {
	series: series::Model,
}

impl HtmlContent for SeriesCard {
	fn fmt(self, formatter: &mut HtmlFormatter) -> fmt::Result {
		write_html!(formatter,
			<div id=self.series.id.to_string() class="shadow-xl card w-96 bg-base-100">
				<div class="card-body">
					<h2 class="card-title">{self.series.name}</h2>
					<p>{self.series.id.to_string()}</p>
				</div>
			</div>
		)
	}
}

async fn index(Db(db): Db) -> Result<impl IntoResponse, WebError> {
	let series = Series::find().all(&db).await?;

	let html = Html::from_fn(|f| {
		write_html!(f,
			<Template title="Series">
				<h1 class="text-4xl font-bold">Series</h1>
				// <ul>
					<For items={series}>
						{ |f, s| SeriesCard { series: s }.fmt(f) }
					</For>
				// </ul>
			</Template>
		)
	});

	Ok(html)
}

pub fn router() -> Router<AppState> {
	Router::new().route("/", get(index))
}
