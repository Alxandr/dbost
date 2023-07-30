use async_trait::async_trait;
use sea_orm::{DatabaseConnection, SqlxPostgresConnector};
use serde::Serialize;
use shuttle_service::{database, DbInput, DbOutput, Factory, ResourceBuilder, Type};
use sqlx::postgres::PgPoolOptions;

#[derive(Serialize)]
pub struct Database {
	config: DbInput,
}

impl Database {
	/// Use a custom connection string for local runs
	pub fn local_uri(mut self, local_uri: &str) -> Self {
		self.config.local_uri = Some(local_uri.to_string());

		self
	}
}

#[async_trait]
impl ResourceBuilder<DatabaseConnection> for Database {
	/// The type of resource this creates
	const TYPE: Type = Type::Database(database::Type::AwsRds(database::AwsRdsEngine::Postgres));

	/// The internal config being constructed by this builder. This will be used to find cached [Self::Output].
	type Config = DbInput;

	/// The output type used to build this resource later
	type Output = DbOutput;

	/// Create a new instance of this resource builder
	fn new() -> Self {
		Self {
			config: DbInput::default(),
		}
	}

	/// Get the internal config state of the builder
	///
	/// If the exact same config was returned by a previous deployement that used this resource, then [Self::output()]
	/// will not be called to get the builder output again. Rather the output state of the previous deployment
	/// will be passed to [Self::build()].
	fn config(&self) -> &Self::Config {
		&self.config
	}

	/// Get the config output of this builder
	///
	/// This method is where the actual resource provisioning should take place and is expected to take the longest. It
	/// can at times even take minutes. That is why the output of this method is cached and calling this method can be
	/// skipped as explained in [Self::config()].
	async fn output(self, factory: &mut dyn Factory) -> Result<Self::Output, shuttle_service::Error> {
		let info = match factory.get_environment() {
			shuttle_service::Environment::Production => DbOutput::Info(
				factory
					.get_db_connection(database::Type::AwsRds(database::AwsRdsEngine::Postgres))
					.await?,
			),
			shuttle_service::Environment::Local => {
				if let Some(local_uri) = self.config.local_uri {
					DbOutput::Local(local_uri)
				} else {
					DbOutput::Info(
						factory
							.get_db_connection(database::Type::AwsRds(database::AwsRdsEngine::Postgres))
							.await?,
					)
				}
			}
		};

		Ok(info)
	}

	/// Build this resource from its config output
	async fn build(build_data: &Self::Output) -> Result<DatabaseConnection, shuttle_service::Error> {
		let connection_string = match build_data {
			DbOutput::Local(local_uri) => local_uri.clone(),
			DbOutput::Info(info) => info.connection_string_private(),
		};

		let pool = PgPoolOptions::new()
			.min_connections(1)
			.max_connections(5)
			.connect(&connection_string)
			.await
			.map_err(shuttle_service::CustomError::new)?;

		Ok(SqlxPostgresConnector::from_sqlx_postgres_pool(pool))
	}
}
