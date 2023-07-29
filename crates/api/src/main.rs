use axum::{extract::State, response::IntoResponse, routing::get, Router};
use axum_healthcheck::{HealthCheck, ResultHealthStatusExt};
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
	db: DatabaseConnection,
}

async fn hello_world() -> &'static str {
	"Hello, world!"
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
	// let check = state.db.ping().await;
	HealthCheck::new()
		.add("db", state.db.ping().await.or_unhealthy("ping failed"))
		.into_response()
}

#[shuttle_runtime::main]
async fn shuttle_main(
	#[shuttle_sea_orm::Database] db: DatabaseConnection,
) -> shuttle_axum::ShuttleAxum {
	axum(db).await
}

async fn axum(db: DatabaseConnection) -> shuttle_axum::ShuttleAxum {
	let state = AppState { db };
	let router = Router::new()
		.route("/healthz", get(health_check))
		.route("/", get(hello_world))
		.with_state(state);

	Ok(router.into())
}
