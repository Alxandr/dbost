use auth::AuthorizationLayer;
use axum::{extract::State, response::IntoResponse, routing::get, Router};
use axum_healthcheck::{HealthCheck, ResultHealthStatusExt};
use migration::MigratorTrait;
use sea_orm::DatabaseConnection;
use shuttle_secrets::SecretStore;
use std::{path::PathBuf, sync::Arc};
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};
use tvdb_client::TvDbClient;

mod api;
mod auth;
mod extractors;
mod web;

#[derive(Clone)]
pub struct AppState {
	db: DatabaseConnection,
	tvdb: Arc<TvDbClient>,
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
	// let check = state.db.ping().await;
	HealthCheck::new()
		.add("db", state.db.ping().await.or_unhealthy("ping failed"))
		.into_response()
}

#[shuttle_runtime::main]
async fn shuttle_main(
	#[shuttle_secrets::Secrets] secret_store: SecretStore,
	#[shuttle_sea_orm::Database(local_uri = "{secrets.DB_CONNECTION_STRING}")] db: DatabaseConnection,
	#[shuttle_static_folder::StaticFolder(folder = "public")] public_folder: PathBuf,
) -> shuttle_axum::ShuttleAxum {
	axum(public_folder, secret_store, db).await
}

async fn axum(
	public_folder: PathBuf,
	secret_store: SecretStore,
	db: DatabaseConnection,
) -> shuttle_axum::ShuttleAxum {
	let api_key = secret_store.get("API_KEY").expect("API_KEY not found");
	let tvdb_api_key = secret_store
		.get("TVDB_API_KEY")
		.expect("TVDB_API_KEY not found");
	let tvdb_user_pin = secret_store
		.get("TVDB_USER_PIN")
		.expect("TVDB_USER_PIN not found");

	migration::Migrator::up(&db, None)
		.await
		.expect("Failed to run migrations");

	let state = AppState {
		db,
		tvdb: Arc::new(TvDbClient::new(tvdb_api_key, tvdb_user_pin).unwrap()),
	};

	let router = Router::new()
		.route("/healthz", get(health_check))
		.nest(
			"/api",
			api::router().layer(AuthorizationLayer::new(api_key)),
		)
		.merge(web::router())
		.with_state(state)
		.nest_service("/public", ServeDir::new(public_folder))
		.layer(CompressionLayer::new())
		.layer(TraceLayer::new_for_http());

	Ok(router.into())
}
