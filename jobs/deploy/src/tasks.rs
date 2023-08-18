// use crate::client::AwsClient;

mod builder;
mod definitions;

pub use builder::TaskDefinitionBuilder;
pub use definitions::dbost_service;
// #[instrument(skip_all)]
// pub async fn dbost(client: &AwsClient) -> Result<TaskDefinitionRevisionId> {
// 	client
// 		.update_task_definition(definitions::dbost_service)
// 		.await
// }
