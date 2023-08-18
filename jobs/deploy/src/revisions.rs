use color_eyre::eyre::{format_err, Context, Result};
use std::{borrow::Cow, ops::Range};
use tracing::{info, instrument, trace};

#[instrument(
	skip_all,
	fields(
		revision.arm = revision.arn(),
		revision.family_name = revision.family_name(),
		revision.revision = revision.revision()
	)
)]
pub async fn derigster_old_revisions(
	client: &aws_sdk_ecs::Client,
	revision: TaskDefinitionRevisionId<'_>,
) -> Result<usize> {
	let definitions = client
		.list_task_definitions()
		.family_prefix(revision.family_name())
		.send()
		.await
		.wrap_err("list task definitions")?
		.task_definition_arns
		.ok_or_else(|| format_err!("no task definitions returned"))?;

	let mut deleted = 0;
	for arn in definitions.iter().map(|a| &**a) {
		let maybe_old_revision = TaskDefinitionRevisionId::try_from(arn)?;
		if maybe_old_revision.family_name != revision.family_name
			|| maybe_old_revision.revision >= revision.revision
		{
			trace!(
				revision.arn = maybe_old_revision.arn(),
				revision.family_name = maybe_old_revision.family_name(),
				revision.revision = maybe_old_revision.revision(),
				"skipping revision"
			);
			continue;
		}

		deregister_revision(client, maybe_old_revision)
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
pub async fn deregister_revision(
	client: &aws_sdk_ecs::Client,
	revision: TaskDefinitionRevisionId<'_>,
) -> Result<()> {
	let arn = revision.arn();
	client
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

#[derive(Debug, Clone)]
pub struct TaskDefinitionRevisionId<'a> {
	arn: Cow<'a, str>,
	family_name: Range<usize>,
	revision: u32,
}

impl<'a> TaskDefinitionRevisionId<'a> {
	pub fn arn(&self) -> &str {
		&self.arn
	}

	pub fn family_name(&self) -> &str {
		&self.arn[self.family_name.clone()]
	}

	pub fn revision(&self) -> u32 {
		self.revision
	}
}

impl<'a> TryFrom<Cow<'a, str>> for TaskDefinitionRevisionId<'a> {
	type Error = color_eyre::eyre::Report;

	fn try_from(arn: Cow<'a, str>) -> Result<Self> {
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
		let family_name = (last_slash + 1)..last_colon;

		Ok(Self {
			arn,
			family_name,
			revision,
		})
	}
}

impl<'a> TryFrom<&'a str> for TaskDefinitionRevisionId<'a> {
	type Error = color_eyre::eyre::Report;

	fn try_from(arn: &'a str) -> Result<Self> {
		let arn = Cow::Borrowed(arn);
		arn.try_into()
	}
}

impl TryFrom<String> for TaskDefinitionRevisionId<'static> {
	type Error = color_eyre::eyre::Report;

	fn try_from(arn: String) -> Result<Self> {
		let arn: Cow<'static, str> = Cow::Owned(arn);
		arn.try_into()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn task_revision_parse() {
		let arn = "arn:aws:ecs:eu-north-1:412850343551:task-definition/dbost:27";

		let id = TaskDefinitionRevisionId::try_from(arn).expect("parse task definition revision id");
		assert_eq!(id.arn(), arn);
		assert_eq!(id.family_name(), "dbost");
		assert_eq!(id.revision(), 27);
	}
}
