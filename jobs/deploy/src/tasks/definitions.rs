use super::builder::{
	ApplicationProtocol, ContainerDefinition, EnvironmentVariable, Port, Secret,
	TaskDefinitionBuilder,
};
use color_eyre::eyre::{Context, Result};

const DB_SCHEMA: EnvironmentVariable = EnvironmentVariable {
	name: "DATABASE_SCHEMA",
	value: "public",
};

const RUST_LOG: EnvironmentVariable = EnvironmentVariable {
	name: "RUST_LOG",
	value: "INFO",
};

pub fn dbost_service(builder: TaskDefinitionBuilder) -> Result<TaskDefinitionBuilder> {
	let migrator = ContainerDefinition {
		name: "dbost-db-migrator",
		image: "ghcr.io/alxandr/dbost/migrator",
		essential: false,
		ro_fs: true,
		memory: 1024,
		ports: [],
		health_check: None,
		depends_on: [],
		env: [DB_SCHEMA, RUST_LOG],
		secrets: [Secret {
			name: "DATABASE_URL",
			secret: "dbost_db_migrator",
			field: "connection_string",
		}],
		log_prefix: "migrator",
	};

	let dbost = ContainerDefinition {
		name: "dbost",
		image: "ghcr.io/alxandr/dbost",
		essential: true,
		ro_fs: true,
		memory: 1024,
		ports: [Port {
			protocol: ApplicationProtocol::Http2,
			container_port: 80,
			name: "www",
		}],
		health_check: Some("http://localhost:80/healthz"),
		depends_on: [migrator.success()],
		env: [
			DB_SCHEMA,
			RUST_LOG,
			EnvironmentVariable {
				name: "SECURE_COOKIES",
				value: "true",
			},
			EnvironmentVariable {
				name: "SELF_URL",
				value: "https://dbost.tv/",
			},
			EnvironmentVariable {
				name: "PORT",
				value: "80",
			},
		],
		secrets: [
			Secret {
				name: "DATABASE_URL",
				secret: "dbost_db_app",
				field: "connection_string",
			},
			Secret {
				name: "SESSION_KEY",
				secret: "dbost_web",
				field: "session_key",
			},
			Secret {
				name: "API_KEY",
				secret: "dbost_web",
				field: "api_key",
			},
			Secret {
				name: "GITHUB_CLIENT_ID",
				secret: "dbost_web",
				field: "github_client_id",
			},
			Secret {
				name: "GITHUB_CLIENT_SECRET",
				secret: "dbost_web",
				field: "github_client_secret",
			},
			Secret {
				name: "TVDB_API_KEY",
				secret: "dbost_tvdb",
				field: "api_key",
			},
			Secret {
				name: "TVDB_USER_PIN",
				secret: "dbost_tvdb",
				field: "user_pin",
			},
		],
		log_prefix: "web",
	};

	builder
		.family("dbost")
		.cpu("512")
		.memory("1024")
		.container(migrator)
		.wrap_err("building task definition 'dbost'")?
		.container(dbost)
		.wrap_err("building task definition 'dbost'")
}

pub fn dbost_cron(builder: TaskDefinitionBuilder) -> Result<TaskDefinitionBuilder> {
	let db_cleaner = ContainerDefinition {
		name: "dbost-db-cleaner",
		image: "ghcr.io/alxandr/dbost/db-cleaner",
		essential: true,
		ro_fs: true,
		memory: 1024,
		ports: [],
		health_check: None,
		depends_on: [],
		env: [DB_SCHEMA, RUST_LOG],
		secrets: [Secret {
			name: "DATABASE_URL",
			secret: "dbost_db_app",
			field: "connection_string",
		}],
		log_prefix: "db-cleaner",
	};

	builder
		.family("dbost-db-cleaner")
		.cpu("512")
		.memory("1024")
		.container(db_cleaner)
		.wrap_err("building task definition 'dbost-db-cleaner'")
}
