use std::env;

use dbost_entities::session;
use dbost_utils::OffsetDateTimeExt;
use sea_orm::{ColumnTrait, ConnectOptions, Database, EntityTrait, QueryFilter};
use time::OffsetDateTime;
use tracing::{debug, warn};

#[tokio::main]
async fn main() {
	_main().await
}

fn required_env_var(name: &str) -> String {
	env::var(name).unwrap_or_else(|_| panic!("${} not found", name))
}

async fn _main() {
	tracing_forest::init();

	let connection_string = required_env_var("DATABASE_URL");
	let db = Database::connect(
		ConnectOptions::new(connection_string)
			// .sqlx_logging(true)
			// .sqlx_logging_level(log::LevelFilter::Info)
			.set_schema_search_path("dbost")
			.to_owned(),
	)
	.await
	.expect("Failed to connect to database");

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
}
