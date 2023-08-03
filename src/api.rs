use crate::AppState;
use axum::Router;

pub(crate) mod series;

pub fn router() -> Router<AppState> {
	Router::new().nest("/series", series::router())
}
