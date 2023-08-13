mod api;
mod auth;
mod extractors;
mod web;

use auth::AuthorizationLayer;
use axum::{
	extract::{FromRef, State},
	response::IntoResponse,
	routing::get,
	Router,
};
use axum_healthcheck::{HealthCheck, ResultHealthStatusExt};
use dbost_entities::session;
use dbost_services::{
	auth::{AuthConfig, GithubAuthConfig},
	series::SeriesService,
};
use dbost_session::{CookieConfig, SessionLayer};
use dbost_utils::OffsetDateTimeExt;
use futures::FutureExt;
use migration::MigratorTrait;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, TransactionTrait};
use shuttle_secrets::SecretStore;
use std::{convert::Infallible, env, path::PathBuf, sync::Arc};
use time::OffsetDateTime;
use tokio::signal::unix::{signal, SignalKind};
use tokio_util::sync::CancellationToken;
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};
use tower_livereload::LiveReloadLayer;
use tracing::{debug, info, info_span, warn, Instrument};
use tvdb_client::TvDbClient;
use url::Url;

#[derive(Clone)]
pub struct AppState {
	db: DatabaseConnection,
	tvdb: Arc<TvDbClient>,
	auth: AuthConfig,
}

impl FromRef<AppState> for DatabaseConnection {
	fn from_ref(input: &AppState) -> Self {
		input.db.clone()
	}
}

impl FromRef<AppState> for AuthConfig {
	fn from_ref(input: &AppState) -> Self {
		input.auth.clone()
	}
}

impl FromRef<AppState> for Arc<TvDbClient> {
	fn from_ref(input: &AppState) -> Self {
		input.tvdb.clone()
	}
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
	let session_key = secret_store
		.get("SESSION_KEY")
		.expect("SESSION_KEY not found");
	let api_key = secret_store.get("API_KEY").expect("API_KEY not found");
	let tvdb_api_key = secret_store
		.get("TVDB_API_KEY")
		.expect("TVDB_API_KEY not found");
	let tvdb_user_pin = secret_store
		.get("TVDB_USER_PIN")
		.expect("TVDB_USER_PIN not found");
	let github_client_id = secret_store
		.get("GITHUB_CLIENT_ID")
		.expect("GITHUB_CLIENT_ID not found");
	let github_client_secret = secret_store
		.get("GITHUB_CLIENT_SECRET")
		.expect("GITHUB_CLIENT_SECRET not found");
	let self_url = secret_store
		.get("SELF_URL")
		.expect("SELF_URL not found")
		.parse::<Url>()
		.expect("SELF_URL must be a valid URL");
	let secure_cookies = secret_store
		.get("SECURE_COOKIES")
		.expect("SECURE_COOKIES not found")
		.parse::<bool>()
		.expect("SECURE_COOKIES must be a boolean");

	// GITHUB_CLIENT_ID = "fd80aa6843145caf7f13"
	// GITHUB_CLIENT_SECRET = "2dc2f959658f8d847d13a168b5bc81bb9984a72c"

	migration::Migrator::up(&db, None)
		.await
		.expect("Failed to run migrations");

	let tvdb = Arc::new(TvDbClient::new(tvdb_api_key, tvdb_user_pin).unwrap());

	let should_seed = env::var("SEED_DATA").unwrap_or_else(|_| "false".to_string()) == "true";
	let should_live_reload =
		env::var("LIVE_RELOAD").unwrap_or_else(|_| "false".to_string()) == "true";

	if should_seed {
		let db = db.clone();
		let tvdb = tvdb.clone();
		tokio::spawn(async move {
			seed_data(db, tvdb.clone())
				.instrument(info_span!("seed data"))
				.await
				.unwrap()
		});
	}

	let auth_service = AuthConfig::builder(db.clone())
		.secure_cookies(secure_cookies)
		.base_path("/auth")
		.with_github(GithubAuthConfig::new(
			github_client_id,
			github_client_secret,
			self_url.join("auth/callback/github").unwrap(),
			["Alxandr"],
		))
		.build();

	let state = AppState {
		db,
		tvdb,
		auth: auth_service,
	};

	let ctrl_c = signal(SignalKind::terminate()).expect("register for ctrl+c failed");
	let ct = CancellationToken::new();
	tokio::spawn({
		let ct = ct.clone();
		async move {
			let mut ctrl_c = ctrl_c;
			ctrl_c.recv().await;
			ct.cancel();
		}
	});

	tokio::spawn({
		let db = state.db.clone();
		let ct = ct.clone();
		async move {
			while !ct.is_cancelled() {
				// cleanup expired sessions
				let result = session::Entity::delete_many()
					.filter(session::Column::Etime.lt(OffsetDateTime::now_utc().into_primitive_utc()))
					.exec(&db)
					.await;

				match result {
					Ok(v) => debug!(
						"deleted {count} expired session rows",
						count = v.rows_affected
					),
					Err(e) => warn!("failed to delete expires sessions: {e:#?}"),
				}

				tokio::select! {
					_ = tokio::time::sleep(tokio::time::Duration::from_secs(60 * 60 /* 1 hour */)) => {},
					_ = ct.cancelled() => { break; },
				}
			}
		}
	});

	let mut router = Router::new()
		.route("/healthz", get(health_check))
		.nest(
			"/api",
			api::router().route_layer(AuthorizationLayer::new(api_key)),
		)
		.merge(web::router().route_layer(SessionLayer::new(
			&session_key,
			CookieConfig {
				secure: secure_cookies,
				domain: None,
				path: "/".into(),
			},
			state.db.clone(),
		)))
		.with_state(state)
		.nest_service("/public", ServeDir::new(public_folder));

	if should_live_reload {
		router = router.layer(LiveReloadLayer::new());
	}

	router = router
		.layer(CompressionLayer::new())
		.layer(TraceLayer::new_for_http());

	Ok(router.into())
}

async fn seed_data(db: DatabaseConnection, tvdb: Arc<TvDbClient>) -> Result<(), DbErr> {
	let service = SeriesService {
		db: db.clone(),
		tvdb,
	};

	db.transaction(move |tx| {
		async move {
			let ids = &[
				259640u64, 267435, 316842, 412374, 352408, 293774, 354167, 102261, 384757, 351953, 377034,
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

			for id in ids.iter().copied() {
				info!(id, "Seeding series");
				service.fetch_from_tvdb(id, Some(tx)).await.unwrap();
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
