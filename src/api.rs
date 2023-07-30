use crate::AppState;
use axum::Router;

mod series;

pub fn router() -> Router<AppState> {
	Router::new().nest("/series", series::router())
}
