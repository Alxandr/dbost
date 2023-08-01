use crate::{extractors::Db, AppState};
use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};
use html_template::{response_html, template as html, HtmlTemplate, HtmlTemplateIterExtensions};
use sea_orm::{EntityTrait, TransactionError};
use std::error;
use theme_db_entities::prelude::*;
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

fn navbar() -> impl HtmlTemplate {
	html! {
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
			</div>

			// search
			<div class="flex-none gap-2">
			<input type="text" placeholder="Search" class="w-24 input input-bordered md:w-auto" />
			</div>
		</nav>
	}
}

async fn index(Db(db): Db) -> Result<impl IntoResponse, WebError> {
	let series = Series::find().all(&db).await?;

	Ok(response_html! {
		<!DOCTYPE html>
		<html>
			<head>
				<meta charset="UTF-8" />
				<title>Series</title>
				<link rel="stylesheet" type="text/css" href="/public/main.css" />
			</head>
			<body>
				{navbar()}
				<h1>Series</h1>
				<ul>
					{series.into_iter().map(|s| html!(<li><a href=format!("/series/{}", s.id)>{s.name}</a></li>)).into_template()}
				</ul>
			</body>
		</html>
	})
}

pub fn router() -> Router<AppState> {
	Router::new().route("/", get(index))
}
