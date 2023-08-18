use crate::{revisions::TaskDefinitionRevisionId, secrets::Secrets, tasks::TaskDefinitionBuilder};
use aws_config::SdkConfig;
use aws_sdk_ecs::types::{Compatibility, Deployment, NetworkMode};
use color_eyre::eyre::{format_err, Context, Result};
use tracing::{info, info_span, instrument, trace, Instrument};

pub struct AwsClient {
	config: SdkConfig,
	ecs: aws_sdk_ecs::Client,
	secrets: Secrets,
	ecs_execution_role: String,
}

impl AwsClient {
	#[instrument(skip_all)]
	pub async fn new(
		config: SdkConfig,
		ecs_execution_role: String,
	) -> color_eyre::eyre::Result<Self> {
		let ecs = aws_sdk_ecs::Client::new(&config);
		let secrets = Secrets::fetch(&config).await?;

		Ok(Self {
			config,
			ecs,
			secrets,
			ecs_execution_role,
		})
	}
}

impl AwsClient {
	#[instrument(
		skip_all,
		fields(
			service.arn = service_arn.as_ref(),
			tag = tag.as_ref(),
		)
	)]
	pub async fn update_service(
		&self,
		service_arn: impl AsRef<str>,
		task_definition_factory: impl for<'a> FnOnce(
			TaskDefinitionBuilder<'a>,
		) -> Result<TaskDefinitionBuilder<'a>>,
		tag: impl AsRef<str>,
	) -> Result<Deployment> {
		let service_arn = service_arn.as_ref();
		let cluster_arn = {
			let last_slash = service_arn
				.rfind('/')
				.ok_or_else(|| format_err!("no slash in service arn"))?;
			&service_arn[..last_slash]
		};

		let revision = self
			.update_task_definition(task_definition_factory, tag.as_ref())
			.await
			.wrap_err("update task definition")?;

		let definition = self.update_service_task_definition(cluster_arn, service_arn, &revision)
			.await
			.wrap_err_with(|| format!("failed to update service task definition for service '{service_arn}' to revision '{revision}'"))?;

		Ok(definition)
	}

	#[instrument(
		skip_all,
		fields(
			cluster.arn = cluster_arn,
			service.arn = service_arn,
			revision.arm = revision.arn(),
			revision.family_name = revision.family_name(),
			revision.revision = revision.revision(),
		)
	)]
	async fn update_service_task_definition(
		&self,
		cluster_arn: &str,
		service_arn: &str,
		revision: &TaskDefinitionRevisionId,
	) -> Result<Deployment> {
		let result = self
			.ecs
			.update_service()
			.cluster(cluster_arn)
			.service(service_arn)
			.task_definition(revision.arn())
			.send()
			.await
			.wrap_err_with(|| {
				format!(
					"failed to update service task definition for service '{service_arn}' to revision '{revision}'",
					service_arn = service_arn,
					revision = revision.arn(),
				)
			})?;

		let deployment = result
			.service
			.ok_or_else(|| format_err!("no service returned after update"))?
			.deployments
			.ok_or_else(|| format_err!("no deployments returned after update"))?
			.into_iter()
			.find(|d| d.task_definition.as_deref() == Some(revision.arn()))
			.ok_or_else(|| {
				format_err!(
					"no deployment for revision '{revision}' returned after update",
					revision = revision.arn()
				)
			})?;

		info!(
			revision.arn = revision.arn(),
			revision.family_name = revision.family_name(),
			revision.revision = revision.revision(),
			deployment.name = deployment.id.as_deref().unwrap_or_default(),
			deployment.status = deployment.status.as_deref().unwrap_or_default(),
			"service successfully updated"
		);

		Ok(deployment)
	}

	#[instrument(skip_all, fields(tag,))]
	async fn update_task_definition(
		&self,
		task_definition_factory: impl for<'a> FnOnce(
			TaskDefinitionBuilder<'a>,
		) -> Result<TaskDefinitionBuilder<'a>>,
		tag: &str,
	) -> Result<TaskDefinitionRevisionId> {
		let region = self
			.config
			.region()
			.ok_or_else(|| format_err!("no region in config"))?;

		let builder = task_definition_factory(TaskDefinitionBuilder::new(
			region,
			tag,
			&self.secrets,
			self
				.ecs
				.register_task_definition()
				.requires_compatibilities(Compatibility::Fargate)
				.network_mode(NetworkMode::Awsvpc)
				.execution_role_arn(&self.ecs_execution_role),
		))?
		.into_inner();

		let family = builder
			.get_family()
			.as_deref()
			.ok_or_else(|| format_err!("no family set in builder"))?;

		let span = info_span!("register task definition", family = family);
		let missing_err = format!("failed to register task definition for family '{family}'");
		let result = span
			.in_scope(|| builder.send())
			.instrument(span)
			.await
			.wrap_err(missing_err)?;

		let new_task_definition = result
			.task_definition
			.ok_or_else(|| format_err!("no task definition returned after update"))?;
		let new_arn = new_task_definition
			.task_definition_arn
			.ok_or_else(|| format_err!("no task definition ARN returned after update"))?;

		let new_revision =
			TaskDefinitionRevisionId::try_from(new_arn).wrap_err("failed to parse new_arn")?;

		self
			.deregister_old_revisions(&new_revision)
			.await
			.wrap_err_with(|| {
				format!(
					"deregister old revisions for {} v{}",
					new_revision.family_name(),
					new_revision.revision()
				)
			})?;

		Ok(new_revision)
	}

	#[instrument(
		skip_all,
		fields(
			revision.arm = revision.arn(),
			revision.family_name = revision.family_name(),
			revision.revision = revision.revision()
		)
	)]
	pub async fn deregister_old_revisions(
		&self,
		revision: &TaskDefinitionRevisionId,
	) -> Result<usize> {
		let definitions = self
			.ecs
			.list_task_definitions()
			.family_prefix(revision.family_name())
			.send()
			.await
			.wrap_err("list task definitions")?
			.task_definition_arns
			.ok_or_else(|| format_err!("no task definitions returned"))?;

		let mut deleted = 0;
		for arn in definitions {
			let maybe_old_revision = TaskDefinitionRevisionId::try_from(arn)?;
			if maybe_old_revision.family_name() != revision.family_name()
				|| maybe_old_revision.revision() >= revision.revision()
			{
				trace!(
					revision.arn = maybe_old_revision.arn(),
					revision.family_name = maybe_old_revision.family_name(),
					revision.revision = maybe_old_revision.revision(),
					"skipping revision"
				);
				continue;
			}

			self
				.deregister_revision(&maybe_old_revision)
				.await
				.wrap_err_with(|| {
					format!(
						"deregister revision {} ({})",
						revision.revision(),
						revision.arn()
					)
				})?;

			deleted += 1;
		}

		Ok(deleted)
	}

	#[instrument(
		skip_all,
		fields(
			revision.arm = revision.arn(),
			revision.family_name = revision.family_name(),
			revision.revision = revision.revision()
		)
	)]
	pub async fn deregister_revision(&self, revision: &TaskDefinitionRevisionId) -> Result<()> {
		let arn = revision.arn();
		self
			.ecs
			.deregister_task_definition()
			.task_definition(arn)
			.send()
			.await
			.wrap_err_with(|| format!("failed to deregister task definition '{arn}'"))?;

		info!(
			revision.arn = revision.arn(),
			revision.family_name = revision.family_name(),
			revision.revision = revision.revision(),
			"revision deregistered"
		);
		Ok(())
	}
}
