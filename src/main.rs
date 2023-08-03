use auth::AuthorizationLayer;
use axum::{extract::State, response::IntoResponse, routing::get, Router};
use axum_healthcheck::{HealthCheck, ResultHealthStatusExt};
use futures::FutureExt;
use migration::MigratorTrait;
use sea_orm::{DatabaseConnection, DbErr, TransactionTrait};
use shuttle_secrets::SecretStore;
use std::{convert::Infallible, env, path::PathBuf, sync::Arc};
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};
use tracing::{info, info_span, Instrument};
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

	let tvdb = Arc::new(TvDbClient::new(tvdb_api_key, tvdb_user_pin).unwrap());
	let should_seed = env::var("SEED_DATA").unwrap_or_else(|_| "false".to_string()) == "true";
	if should_seed {
		seed_data(&db, tvdb.clone())
			.instrument(info_span!("seed data"))
			.await
			.unwrap();
	}

	let state = AppState { db, tvdb };

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

async fn seed_data(db: &DatabaseConnection, tvdb: Arc<TvDbClient>) -> Result<(), DbErr> {
	db.transaction(move |tx| {
		async move {
			let ids = &[
				259640, 267435, 316842, 412374, 352408, 293774, 354167, 102261, 384757, 351953, 377034,
				289884, 316931, 341425, 386714, 272128, 295685, 294002, 355774, 362429, 327007, 339268,
				289882, 387391, 337020, 341432, 289886, 326109, 406592, 278626, 360295, 305089, 321869,
				370377, 332984, 284719, 378879, 386818, 293088, 414057, 355567, 342117, 305074, 337018,
				328827, 345596, 348545, 357492, 357019, 368358, 337017, 353666, 416802, 78804, 332771,
				330139, 346942, 303867, 306111, 361491, 321535, 353712, 357864, 355480, 357888, 79880,
				384541, 397934, 407520, 264663, 416902, 425520, 77680, 421378, 402607, 423362, 404525,
				415188, 419126, 420657, 402474, 427239, 421737, 424435, 426165, 421069, 361013, 244061,
				283937, 428108, 423121, 413333, 359274, 422090, 410425, 418364, 416359, 429310, 413578,
				259647,
			];

			for id in ids {
				info!(id, "Seeding series");
				api::series::seed_tbdb_id(*id, tx, &tvdb).await.unwrap();
			}

			let result: Result<(), Infallible> = Ok(());
			result
		}
		.boxed()
	})
	.await
	.unwrap();

	Ok(())
}
