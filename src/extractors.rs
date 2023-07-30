use axum::{async_trait, extract::FromRequestParts, http::request};
use sea_orm::DatabaseConnection;
use std::{convert::Infallible, sync::Arc};
use tvdb_client::TvDbClient;

use crate::AppState;

pub struct Db(pub DatabaseConnection);

#[async_trait]
impl FromRequestParts<AppState> for Db {
	type Rejection = Infallible;

	async fn from_request_parts(
		_parts: &mut request::Parts,
		state: &AppState,
	) -> Result<Self, Self::Rejection> {
		Ok(Db(state.db.clone()))
	}
}

pub struct TvDb(pub Arc<TvDbClient>);

#[async_trait]
impl FromRequestParts<AppState> for TvDb {
	type Rejection = Infallible;

	async fn from_request_parts(
		_parts: &mut request::Parts,
		state: &AppState,
	) -> Result<Self, Self::Rejection> {
		Ok(TvDb(state.tvdb.clone()))
	}
}
