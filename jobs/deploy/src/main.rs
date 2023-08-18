use std::collections::HashMap;

use aws_config::AppName;
use aws_sdk_ecs::types::{
	builders::ContainerDefinitionBuilder, ApplicationProtocol, Compatibility, ContainerCondition,
	ContainerDefinition, ContainerDependency, HealthCheck, KeyValuePair, LogConfiguration, LogDriver,
	NetworkMode, PortMapping,
};
use aws_sdk_secretsmanager::types::SecretListEntry;
use clap::Parser;
use color_eyre::eyre::{format_err, Context, Result};
use tracing::{debug, info, metadata::LevelFilter};
use tracing_forest::ForestLayer;
use tracing_subscriber::{prelude::*, EnvFilter};

const REGION: &str = "eu-north-1";

/// CLI to deploy new version to AWS
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Tag for the new images
	#[arg(short, long, env = "TAG")]
	tag: String,
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;
	tracing_subscriber::registry()
		.with(ForestLayer::default())
		.with(
			EnvFilter::builder()
				.with_default_directive(LevelFilter::INFO.into())
				.from_env_lossy(),
		)
		.init();

	_main().await
}

async fn _main() -> Result<()> {
	let args = Args::parse();
	let config = aws_config::from_env()
		.region(REGION)
		.app_name(AppName::new("dbost-deploy").unwrap())
		.load()
		.await;

	info!(
		region = ?config.region(),
		endpoint_url = ?config.endpoint_url(),
		retry_config = ?config.retry_config(),
		app_name = ?config.app_name(),
		use_fips = ?config.use_fips(),
		use_dual_stack = ?config.use_dual_stack(),
		"loaded config"
	);

	let client = aws_sdk_ecs::Client::new(&config);
	let secrets = SecretManager::fetch(&config)
		.await
		.wrap_err("falied to get secrets")?;
	let tag = args.tag;

	let result = client
		.register_task_definition()
		.family("dbost")
		.requires_compatibilities(Compatibility::Fargate)
		.network_mode(NetworkMode::Awsvpc)
		.cpu("512")
		.memory("1024")
		.execution_role_arn("arn:aws:iam::412850343551:role/ecs-agent")
		.container_definitions(
			ContainerDefinition::builder()
				.name("dbost-db-migrator")
				.image(format!("ghcr.io/alxandr/dbost/migrator:{}", tag))
				.essential(false)
				.readonly_root_filesystem(true)
				.memory(1024)
				.env("DATABASE_SCHEMA", "public")
				.env("RUST_LOG", "INFO")
				.secret(
					"DATABASE_URL",
					secrets.get("dbost_db_migrator")?,
					"connection_string",
				)
				.log_configuration(
					LogConfiguration::builder()
						.log_driver(LogDriver::Awslogs)
						.options("awslogs-create-group", "true")
						.options("awslogs-group", "dbost")
						.options("awslogs-region", REGION)
						.options("awslogs-stream-prefix", "migrator")
						.build(),
				)
				.build(),
		)
		.container_definitions(
			ContainerDefinition::builder()
				.name("dbost")
				.image(format!("ghcr.io/alxandr/dbost:{}", tag))
				.readonly_root_filesystem(true)
				.memory(1024)
				.port_mappings(
					PortMapping::builder()
						.app_protocol(ApplicationProtocol::Http2)
						.container_port(80)
						.name("www")
						.build(),
				)
				.health_check(
					HealthCheck::builder()
						.command("CMD-SHELL")
						.command("curl -f http://localhost:80/healthz || exit 1")
						.start_period(2)
						.build(),
				)
				.depends_on(
					ContainerDependency::builder()
						.container_name("dbost-db-migrator")
						.condition(ContainerCondition::Success)
						.build(),
				)
				.env("DATABASE_SCHEMA", "public")
				.env("RUST_LOG", "INFO")
				.env("SECURE_COOKIES", "true")
				.env("SELF_URL", "https://dbost.tv/")
				.env("PORT", "80")
				.secret(
					"DATABASE_URL",
					secrets.get("dbost_db_app")?,
					"connection_string",
				)
				.secret("SESSION_KEY", secrets.get("dbost_web")?, "session_key")
				.secret("API_KEY", secrets.get("dbost_web")?, "api_key")
				.secret(
					"GITHUB_CLIENT_ID",
					secrets.get("dbost_web")?,
					"github_client_id",
				)
				.secret(
					"GITHUB_CLIENT_SECRET",
					secrets.get("dbost_web")?,
					"github_client_secret",
				)
				.secret("TVDB_API_KEY", secrets.get("dbost_tvdb")?, "api_key")
				.secret("TVDB_USER_PIN", secrets.get("dbost_tvdb")?, "user_pin")
				.log_configuration(
					LogConfiguration::builder()
						.log_driver(LogDriver::Awslogs)
						.options("awslogs-create-group", "true")
						.options("awslogs-group", "dbost")
						.options("awslogs-region", REGION)
						.options("awslogs-stream-prefix", "web")
						.build(),
				)
				.build(),
		)
		.send()
		.await
		.wrap_err("update task definition")?;

	let new_task_definition = result
		.task_definition
		.ok_or_else(|| format_err!("no task definition returned after update"))?;
	let new_arn = new_task_definition
		.task_definition_arn
		.ok_or_else(|| format_err!("no task definition ARN returned after update"))?;

	let new_revision =
		TaskDefinitionRevisionId::try_from(&*new_arn).wrap_err("failed to parse new_arn")?;

	info!(
		revision.arn = new_revision.arn,
		revision.family_name = new_revision.family_name,
		revision.revision = new_revision.revision,
		"new revision created"
	);

	let definitions = client
		.list_task_definitions()
		.family_prefix(new_revision.family_arn)
		.send()
		.await
		.wrap_err("list task definitions")?
		.task_definition_arns
		.ok_or_else(|| format_err!("no task definitions returned"))?;

	for arn in definitions {
		let existing_revision = TaskDefinitionRevisionId::try_from(&*arn)?;
		if existing_revision.family_name != new_revision.family_name
			|| existing_revision.revision >= new_revision.revision
		{
			info!(
				revision.arn = existing_revision.arn,
				revision.family_name = existing_revision.family_name,
				revision.revision = existing_revision.revision,
				"skipping revision"
			);
			continue;
		}

		info!(
			revision.arn = existing_revision.arn,
			revision.family_name = existing_revision.family_name,
			revision.revision = existing_revision.revision,
			"deregistering old revision"
		);
	}

	client
		.update_service()
		.cluster("arn:aws:ecs:eu-north-1:412850343551:cluster/dbost-cluster")
		.service("arn:aws:ecs:eu-north-1:412850343551:service/dbost-cluster/dbost")
		.task_definition(new_arn)
		.send()
		.await
		.wrap_err("update service")?;

	info!(tag, "service successfully updated");

	Ok(())
}

struct SecretManager {
	secrets: HashMap<String, Secret>,
}

impl SecretManager {
	async fn fetch(config: &aws_config::SdkConfig) -> Result<Self> {
		let client = aws_sdk_secretsmanager::Client::new(config);

		let secrets = client
			.list_secrets()
			.send()
			.await
			.wrap_err("list secrets")?;

		let secrets = secrets
			.secret_list
			.ok_or_else(|| format_err!("no secrets returned"))?
			.into_iter()
			.map(Secret::try_from)
			.map(|s| s.map(|s| (s.name.clone(), s)))
			.collect::<Result<_>>()?;

		Ok(Self { secrets })
	}

	fn get(&self, name: impl AsRef<str>) -> Result<&Secret> {
		let name = name.as_ref();
		self
			.secrets
			.get(name)
			.ok_or_else(|| format_err!("secret {name} not found"))
			.map_err(Into::into)
	}
}

#[derive(Debug, Clone)]
struct TaskDefinitionRevisionId<'a> {
	arn: &'a str,
	family_name: &'a str,
	family_arn: &'a str,
	revision: u32,
}

impl<'a> TryFrom<&'a str> for TaskDefinitionRevisionId<'a> {
	type Error = color_eyre::eyre::Report;

	fn try_from(arn: &'a str) -> Result<Self> {
		let last_colon = arn
			.rfind(':')
			.ok_or_else(|| format_err!("missing : in ARN"))?;
		let family_arn = &arn[..last_colon];
		let revision = arn[(last_colon + 1)..]
			.parse::<u32>()
			.wrap_err("parse task definition revision id")?;

		let last_slash = family_arn
			.rfind('/')
			.ok_or_else(|| format_err!("missing / in ARN"))?;
		let family_name = &family_arn[(last_slash + 1)..];

		Ok(Self {
			arn,
			family_arn,
			family_name,
			revision,
		})
	}
}

struct Secret {
	name: String,
	arn: String,
	current: String,
}

impl Secret {
	fn field(&self, field: impl AsRef<str>) -> String {
		let field = field.as_ref();
		let Secret { arn, current, .. } = self;
		format!("{arn}:{field}::{current}")
	}
}

impl TryFrom<SecretListEntry> for Secret {
	type Error = color_eyre::eyre::Report;

	fn try_from(value: SecretListEntry) -> std::result::Result<Self, Self::Error> {
		let name = value.name.ok_or_else(|| format_err!("no name returned"))?;
		debug!(name, "found secret");

		let arn = value
			.arn
			.ok_or_else(|| format_err!("no ARN returned for {name}"))?;

		let versions = value
			.secret_versions_to_stages
			.ok_or_else(|| format_err!("no versions returned for {name}"))?;

		let current = versions
			.into_iter()
			.find(|(_, stage)| stage.iter().any(|stage| stage == "AWSCURRENT"))
			.map(|(version, _)| version)
			.ok_or_else(|| format_err!("no current version found for {name}"))?;

		Ok(Self { name, arn, current })
	}
}

trait ContainerDefinitionBuilderExt {
	fn env(self, name: impl Into<String>, value: impl Into<String>) -> Self;
	fn secret(self, name: impl Into<String>, secret: &Secret, field: impl AsRef<str>) -> Self;
}

impl ContainerDefinitionBuilderExt for ContainerDefinitionBuilder {
	fn env(self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.environment(KeyValuePair::builder().name(name).value(value).build())
	}

	fn secret(self, name: impl Into<String>, secret: &Secret, field: impl AsRef<str>) -> Self {
		self.secrets(
			aws_sdk_ecs::types::Secret::builder()
				.name(name)
				.value_from(secret.field(field))
				.build(),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn task_revision_parse() {
		let arn = "arn:aws:ecs:eu-north-1:412850343551:task-definition/dbost:27";

		let id = TaskDefinitionRevisionId::try_from(arn).expect("parse task definition revision id");
		assert_eq!(id.arn, arn);
		assert_eq!(
			id.family_arn,
			"arn:aws:ecs:eu-north-1:412850343551:task-definition/dbost"
		);
		assert_eq!(id.family_name, "dbost");
		assert_eq!(id.revision, 27);
	}
}
