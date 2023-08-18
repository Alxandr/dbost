mod client;
mod revisions;
mod secrets;
mod tasks;

use crate::client::AwsClient;
use aws_config::AppName;
use clap::Parser;
use color_eyre::eyre::{Context, Result};
use tracing::{info, metadata::LevelFilter};
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

	let client = AwsClient::new(config, "arn:aws:iam::412850343551:role/ecs-agent".into()).await?;
	client
		.update_service(
			"arn:aws:ecs:eu-north-1:412850343551:service/dbost-cluster/dbost",
			tasks::dbost_service,
			&args.tag,
		)
		.await
		.wrap_err("update dbost service")?;

	client
		.register_task_definition(tasks::dbost_cron, &args.tag)
		.await
		.wrap_err("update dbost cron task definition")?;

	Ok(())
}
