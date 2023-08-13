use async_trait::async_trait;
use reqwest::{
	header::{self, HeaderMap},
	Client, Request, Response,
};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Middleware, Next};
use task_local_extensions::Extensions;
use thiserror::Error;
use tracing::{info, info_span, Instrument};

mod auth;
mod series;

pub static PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub static PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub use series::{Season, Series};

#[derive(Error, Debug)]
pub enum TvDbError {
	#[error("Request error: {0}")]
	RequestError(#[from] reqwest_middleware::Error),
}

impl From<reqwest::Error> for TvDbError {
	fn from(value: reqwest::Error) -> Self {
		Self::RequestError(value.into())
	}
}

enum TvDbUrl {
	Login,
	Series(u64),
	Season(u64),
}

impl TvDbUrl {
	fn into_url(self) -> reqwest::Url {
		let path = match self {
			Self::Login => "login".to_owned(),
			Self::Series(id) => format!("series/{id}/extended?meta=translations"),
			Self::Season(id) => format!("seasons/{id}/extended?meta=translations"),
		};

		reqwest::Url::parse(&format!("https://api4.thetvdb.com/v4/{path}")).unwrap()
	}
}

pub struct TvDbClient {
	client: ClientWithMiddleware,
}

impl TvDbClient {
	pub fn new(api_key: String, user_pin: String) -> Result<Self, reqwest::Error> {
		let mut headers = HeaderMap::new();
		headers.insert(
			header::ACCEPT,
			header::HeaderValue::from_static("application/json"),
		);

		Ok(Self {
			client: ClientBuilder::new(
				Client::builder()
					.user_agent(APP_USER_AGENT)
					.pool_idle_timeout(std::time::Duration::from_secs(5))
					.pool_max_idle_per_host(2)
					.default_headers(headers)
					.build()?,
			)
			.with(auth::AuthMiddleware::new(api_key, user_pin))
			.with(TracingMiddleware)
			.build(),
		})
	}

	pub async fn get_series(&self, id: u64) -> Result<Option<series::Series>, TvDbError> {
		series::get_series(id, self).await
	}
}

struct TracingMiddleware;

#[async_trait]
impl Middleware for TracingMiddleware {
	async fn handle(
		&self,
		req: Request,
		extensions: &mut Extensions,
		next: Next<'_>,
	) -> reqwest_middleware::Result<Response> {
		let span = info_span!(
			"request",
			method = %req.method(),
			uri = %req.url(),
			authorized = req.headers().get("authorization").is_some()
		);

		span
			.in_scope(|| {
				info!(
					method = %req.method(),
					uri = %req.url(),
					authorized = req.headers().get("authorization").is_some(),
					"sending request",
				);
				next.run(req, extensions)
			})
			.instrument(span.clone())
			.await
	}
}
