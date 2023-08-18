use color_eyre::eyre::{format_err, Context, Result};
use std::{fmt, ops::Range};

#[derive(Debug, Clone)]
pub struct TaskDefinitionRevisionId {
	arn: String,
	family_name: Range<usize>,
	revision: u32,
}

impl fmt::Display for TaskDefinitionRevisionId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&self.arn, f)
	}
}

impl TaskDefinitionRevisionId {
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

impl TryFrom<String> for TaskDefinitionRevisionId {
	type Error = color_eyre::eyre::Report;

	fn try_from(arn: String) -> Result<Self> {
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn task_revision_parse() {
		let arn = "arn:aws:ecs:eu-north-1:412850343551:task-definition/dbost:27";

		let id = TaskDefinitionRevisionId::try_from(arn.to_owned())
			.expect("parse task definition revision id");
		assert_eq!(id.arn(), arn);
		assert_eq!(id.family_name(), "dbost");
		assert_eq!(id.revision(), 27);
	}
}
