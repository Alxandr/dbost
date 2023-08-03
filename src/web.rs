use crate::{extractors::Db, AppState};
use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};
use html_template::{component, HtmlTemplate, HtmlTemplateResponseExt, IntoHtmlTemplate};
use sea_orm::{EntityTrait, TransactionError};
use std::error;
use theme_db_entities::{prelude::*, series};
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

component! {
	struct NavBar {
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

component! {
	struct Template(title: impl Into<String>, children: impl HtmlTemplate) {
		<!DOCTYPE html>
		<html>
			<head>
				<meta charset="UTF-8" />
				<title>{title.into()}</title>
				<link rel="stylesheet" type="text/css" href="/public/main.css" />
			</head>
			<body>
				<NavBar />
				<main>
					{children}
				</main>
			</body>
		</html>
	}
}

component! {
	struct IndexPage(items: impl HtmlTemplate) {
		<Template title="Series">
			<h1>Series</h1>
			<ul>
				{items}
			</ul>
		</Template>
	}
}

component! {
	struct SeriesIndexListItem(series: series::Model) {
		<li>
			<a href=format!("/series/{}", series.id)>{series.name}</a>
		</li>
	}
}

async fn index(Db(db): Db) -> Result<impl IntoResponse, WebError> {
	let series = Series::find().all(&db).await?;

	Ok(
		IndexPage::new(
			series
				.into_iter()
				.map(SeriesIndexListItem::new)
				.into_template(),
		)
		.into_response(),
	)
}

pub fn router() -> Router<AppState> {
	Router::new().route("/", get(index))
}

trait FancyComp: Sized {
	type Props: Into<Self> + Sized;
}

struct MyComp(String);

const _: () = {
	struct FancyCompProps {
		name: String,
	}

	impl FancyComp for MyComp {
		type Props = FancyCompProps;
	}

	impl From<FancyCompProps> for MyComp {
		fn from(val: FancyCompProps) -> Self {
			MyComp(val.name)
		}
	}
};

fn test() {
	type Props<T: FancyComp> = <T as FancyComp>::Props;
	let comp: MyComp = Props::<MyComp> {
		name: "hello".to_string(),
	}
	.into();
}

// Fake syntax:
// component! {
// 	struct Template(title: impl Into<String>, children: impl HtmlTemplate) {
// 		<!DOCTYPE html>
// 		<html>
// 			<head>
// 				<meta charset="UTF-8" />
// 				<title>{title.into()}</title>
// 				<link rel="stylesheet" type="text/css" href="/public/main.css" />
// 			</head>
// 			<body>
// 				<NavBar />
// 				<main>
// 					{children}
// 				</main>
// 			</body>
// 		</html>
// 	}
// }

// #[derive(HtmlComponent)]
// struct Template<Title, Children> {
// 	title: Title,
// 	children: Children,
// }

// impl<Title, Children> RenderHtmlComponent for Template<Title, Children>
// where
// 	Title: Into<String>,
// 	Children: HtmlTemplate,
// {
// 	fn render(self, formatter: HtmlFormatter) -> fmt::Result {
// 		write_html!(formatter,
// 			<!DOCTYPE html>
// 			<html>
// 				<head>
// 					<meta charset="UTF-8" />
// 					<title>{self.title.into()}</title>
// 					<link rel="stylesheet" type="text/css" href="/public/main.css" />
// 				</head>
// 				<body>
// 					<NavBar />
// 					<main>
// 						{self.children}
// 					</main>
// 				</body>
// 			</html>
// 		)
// 	}
// }
