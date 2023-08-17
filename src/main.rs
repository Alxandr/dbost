mod api;
mod auth;
mod extractors;
mod web;

mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use auth::AuthorizationLayer;
use axum::{
	extract::{FromRef, State},
	response::IntoResponse,
	routing::get,
	Router,
};
use axum_healthcheck::{HealthCheck, ResultHealthStatusExt};
use dbost_services::auth::{AuthConfig, GithubAuthConfig};
use dbost_session::{CookieConfig, SessionLayer};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::{
	env,
	net::{SocketAddr, SocketAddrV4},
	process::exit,
	sync::Arc,
};
use tokio::signal::unix::{signal, SignalKind};
use tokio_util::sync::CancellationToken;
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};
use tracing::{info, metadata::LevelFilter};
use tracing_forest::ForestLayer;
use tracing_subscriber::{prelude::*, EnvFilter};
use tvdb_client::TvDbClient;
use url::Url;

#[cfg(feature = "live-reload")]
use tower_livereload::LiveReloadLayer;

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

#[tokio::main]
async fn main() -> ! {
	axum().await
}

fn required_env_var(name: &str) -> String {
	env::var(name).unwrap_or_else(|_| panic!("${} not found", name))
}

async fn axum() -> ! {
	tracing_subscriber::registry()
		.with(ForestLayer::default())
		.with(
			EnvFilter::builder()
				.with_default_directive(LevelFilter::INFO.into())
				.from_env_lossy(),
		)
		.init();

	info!(
		version = built_info::PKG_VERSION,
		profile = built_info::PROFILE,
		git.sha = option_env!("GIT_SHA"),
		"dbost info"
	);

	let connection_string = required_env_var("DATABASE_URL");
	let database_schema = required_env_var("DATABASE_SCHEMA");
	let web_public_path = required_env_var("WEB_PUBLIC_PATH");
	let session_key = required_env_var("SESSION_KEY");
	let api_key = required_env_var("API_KEY");
	let tvdb_api_key = required_env_var("TVDB_API_KEY");
	let tvdb_user_pin = required_env_var("TVDB_USER_PIN");
	let github_client_id = required_env_var("GITHUB_CLIENT_ID");
	let github_client_secret = required_env_var("GITHUB_CLIENT_SECRET");
	let self_url = required_env_var("SELF_URL")
		.parse::<Url>()
		.expect("SELF_URL must be a valid URL");
	let secure_cookies = required_env_var("SECURE_COOKIES")
		.parse::<bool>()
		.expect("SECURE_COOKIES must be a boolean");

	let db = Database::connect(
		ConnectOptions::new(connection_string)
			// .sqlx_logging(true)
			// .sqlx_logging_level(log::LevelFilter::Info)
			.set_schema_search_path(database_schema)
			.to_owned(),
	)
	.await
	.expect("Failed to connect to database");
	let tvdb = Arc::new(TvDbClient::new(tvdb_api_key, tvdb_user_pin).unwrap());

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
		.nest_service("/public", ServeDir::new(web_public_path));

	#[cfg(feature = "live-reload")]
	{
		router = router.layer(LiveReloadLayer::new());
	}

	router = router
		.layer(CompressionLayer::new())
		.layer(TraceLayer::new_for_http());

	let port = env::var("PORT")
		.map(|v| v.parse::<u16>().expect("PORT must be a valid port number"))
		.unwrap_or(8000);

	let addr = SocketAddrV4::new([0, 0, 0, 0].into(), port);
	let err = axum::Server::bind(&SocketAddr::V4(addr))
		.serve(router.into_make_service())
		.await // runs forever(ish)
		.unwrap_err();

	eprintln!("server failed: {err}");
	exit(1);
}
