use aws_sdk_ecs::{
	config::Region,
	operation::register_task_definition::builders::RegisterTaskDefinitionFluentBuilder,
	types::{
		ContainerDefinition as AwsContainerDefinition, KeyValuePair, LogConfiguration, PortMapping,
	},
	types::{HealthCheck, LogDriver},
};
use color_eyre::eyre::{Context, Result};

use crate::secrets::Secrets;

#[derive(Clone, Copy)]
pub enum ApplicationProtocol {
	Http2,
}

impl From<ApplicationProtocol> for aws_sdk_ecs::types::ApplicationProtocol {
	fn from(protocol: ApplicationProtocol) -> Self {
		match protocol {
			ApplicationProtocol::Http2 => Self::Http2,
		}
	}
}

#[derive(Clone, Copy)]
pub enum ContainerCondition {
	Success,
}

impl From<ContainerCondition> for aws_sdk_ecs::types::ContainerCondition {
	fn from(condition: ContainerCondition) -> Self {
		match condition {
			ContainerCondition::Success => Self::Success,
		}
	}
}

#[derive(Clone, Copy)]
pub struct EnvironmentVariable {
	pub name: &'static str,
	pub value: &'static str,
}

#[derive(Clone, Copy)]
pub struct Secret {
	pub name: &'static str,
	pub secret: &'static str,
	pub field: &'static str,
}

#[derive(Clone, Copy)]
pub struct Port {
	pub protocol: ApplicationProtocol,
	pub container_port: u16,
	pub name: &'static str,
}

#[derive(Clone, Copy)]
pub struct ContainerDependency {
	pub container: &'static str,
	pub condition: ContainerCondition,
}

#[derive(Clone, Copy)]
pub struct ContainerDefinition<
	const ENV: usize,
	const SECRET: usize,
	const PORT: usize,
	const DEP: usize,
> {
	pub name: &'static str,
	pub image: &'static str,
	pub essential: bool,
	pub ro_fs: bool,
	pub memory: u16,
	pub ports: [Port; PORT],
	pub health_check: Option<&'static str>, // uri to curl
	pub depends_on: [ContainerDependency; DEP],
	pub env: [EnvironmentVariable; ENV],
	pub secrets: [Secret; SECRET],
	pub log_prefix: &'static str,
}

impl<const ENV: usize, const SECRET: usize, const PORT: usize, const DEP: usize>
	ContainerDefinition<ENV, SECRET, PORT, DEP>
{
	pub fn success(&self) -> ContainerDependency {
		ContainerDependency {
			container: self.name,
			condition: ContainerCondition::Success,
		}
	}
}

pub struct TaskDefinitionBuilder<'a> {
	region: &'a Region,
	tag: &'a str,
	secrets: &'a Secrets,
	builder: RegisterTaskDefinitionFluentBuilder,
}

impl<'a> TaskDefinitionBuilder<'a> {
	pub fn new(
		region: &'a Region,
		tag: &'a str,
		secrets: &'a Secrets,
		builder: RegisterTaskDefinitionFluentBuilder,
	) -> Self {
		Self {
			region,
			tag,
			secrets,
			builder,
		}
	}

	pub fn into_inner(self) -> RegisterTaskDefinitionFluentBuilder {
		self.builder
	}

	pub fn family(self, family: &'a str) -> Self {
		Self {
			region: self.region,
			tag: self.tag,
			secrets: self.secrets,
			builder: self.builder.family(family),
		}
	}

	pub fn cpu(self, cpu: &'static str) -> Self {
		Self {
			region: self.region,
			tag: self.tag,
			secrets: self.secrets,
			builder: self.builder.cpu(cpu),
		}
	}

	pub fn memory(self, memory: &'static str) -> Self {
		Self {
			region: self.region,
			tag: self.tag,
			secrets: self.secrets,
			builder: self.builder.memory(memory),
		}
	}

	pub fn container<const ENV: usize, const SECRET: usize, const PORT: usize, const DEP: usize>(
		self,
		def: ContainerDefinition<ENV, SECRET, PORT, DEP>,
	) -> Result<Self> {
		let tag = self.tag;
		let ContainerDefinition {
			name,
			image,
			essential,
			ro_fs,
			memory,
			ports,
			health_check,
			depends_on,
			env,
			secrets: secret,
			log_prefix,
		} = def;

		let mut builder = AwsContainerDefinition::builder()
			.name(name)
			.image(format!("{image}:{tag}"))
			.essential(essential)
			.readonly_root_filesystem(ro_fs)
			.memory(memory as i32)
			.log_configuration(
				LogConfiguration::builder()
					.log_driver(LogDriver::Awslogs)
					.options("awslogs-create-group", "true")
					.options("awslogs-group", "dbost")
					.options("awslogs-region", self.region.as_ref())
					.options("awslogs-stream-prefix", log_prefix)
					.build(),
			)
			.set_health_check(health_check.map(|uri| {
				HealthCheck::builder()
					.set_command(Some(vec![
						"CMD-SHELL".to_owned(),
						format!("curl -f \"{uri}\" || exit 1"),
					]))
					.start_period(2)
					.build()
			}));

		for ContainerDependency {
			container,
			condition,
		} in depends_on
		{
			builder = builder.depends_on(
				aws_sdk_ecs::types::ContainerDependency::builder()
					.container_name(container)
					.condition(condition.into())
					.build(),
			);
		}

		for Port {
			protocol,
			container_port,
			name,
		} in ports
		{
			builder = builder.port_mappings(
				PortMapping::builder()
					.app_protocol(protocol.into())
					.container_port(container_port as i32)
					.name(name)
					.build(),
			);
		}

		for EnvironmentVariable { name, value } in env {
			builder = builder.environment(KeyValuePair::builder().name(name).value(value).build());
		}

		for Secret {
			name,
			secret,
			field,
		} in secret
		{
			let secret = self
				.secrets
				.get(secret)
				.wrap_err_with(|| format!("building container definition for '{name}'"))?
				.field(field);
			builder = builder.secrets(
				aws_sdk_ecs::types::Secret::builder()
					.name(name)
					.value_from(secret)
					.build(),
			);
		}

		Ok(Self {
			tag: self.tag,
			region: self.region,
			secrets: self.secrets,
			builder: self.builder.container_definitions(builder.build()),
		})
	}
}
