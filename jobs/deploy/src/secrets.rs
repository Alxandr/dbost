use aws_sdk_secretsmanager::types::SecretListEntry;
use color_eyre::eyre::{format_err, Context, Result};
use std::collections::HashMap;
use tracing::{debug, instrument};

pub struct Secrets {
	secrets: HashMap<String, Secret>,
}

impl Secrets {
	#[instrument(skip_all)]
	pub async fn fetch(config: &aws_config::SdkConfig) -> Result<Self> {
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

	pub fn get(&self, name: impl AsRef<str>) -> Result<&Secret> {
		let name = name.as_ref();
		self
			.secrets
			.get(name)
			.ok_or_else(|| format_err!("secret {name} not found"))
			.map_err(Into::into)
	}
}

pub struct Secret {
	name: String,
	arn: String,
	current: String,
}

impl Secret {
	pub fn field(&self, field: impl AsRef<str>) -> String {
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
