use dbost_services::series::SeriesService;
use futures::FutureExt;
use sea_orm::{ConnectOptions, Database, TransactionTrait};
use std::{convert::Infallible, env, sync::Arc};
use tracing::{info, metadata::LevelFilter};
use tracing_forest::ForestLayer;
use tracing_subscriber::{prelude::*, EnvFilter};
use tvdb_client::TvDbClient;

#[tokio::main]
async fn main() {
	_main().await
}

fn required_env_var(name: &str) -> String {
	env::var(name).unwrap_or_else(|_| panic!("${} not found", name))
}

async fn _main() {
	tracing_subscriber::registry()
		.with(ForestLayer::default())
		.with(
			EnvFilter::builder()
				.with_default_directive(LevelFilter::INFO.into())
				.from_env_lossy(),
		)
		.init();

	let connection_string = required_env_var("DATABASE_URL");
	let database_schema = required_env_var("DATABASE_SCHEMA");
	let tvdb_api_key = required_env_var("TVDB_API_KEY");
	let tvdb_user_pin = required_env_var("TVDB_USER_PIN");

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
}
