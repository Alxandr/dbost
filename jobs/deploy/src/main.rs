use clap::Parser;
use color_eyre::eyre::{bail, format_err, Context, Result};
use paste::paste;

/// CLI to deploy new version to AWS
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Tag for the new images
	#[arg(short, long)]
	tag: String,
}

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;
	_main().await
}

macro_rules! copy_response_to_request {
		($res:ident => $req:expr ; [$($fld:ident),+$(,)?]) => {
			paste!{
				$req
					$(. [<set_ $fld>]($res.$fld))+
			}
		};
}

async fn _main() -> Result<()> {
	let args = Args::parse();
	let config = aws_config::load_from_env().await;
	let client = aws_sdk_ecs::Client::new(&config);
	let tag = args.tag;

	let service_task_definition_response = client
		.describe_task_definition()
		.task_definition("arn:aws:ecs:eu-north-1:412850343551:task-definition/dbost")
		.send()
		.await
		.wrap_err("get task definition")?;

	let Some(mut service_task_definition) = service_task_definition_response.task_definition else {
		bail!("no task definition");
	};

	match service_task_definition.container_definitions.as_deref_mut() {
		None => bail!("no container definitions"),
		Some(c) => {
			for container in c {
				let Some(name) = container.name.as_deref() else { bail!("no container name") };
				let Some(image) = container.image.as_mut() else { bail!("no image for container {name}") };
				let new = image.replace(":latest", &format!(":{tag}"));
				println!("{}: {} -> {}", name, image, new);
				*image = new;
			}
		}
	}

	let result =
		copy_response_to_request!(service_task_definition => client.register_task_definition(); [
			container_definitions,
			family,
			task_role_arn,
			execution_role_arn,
			network_mode,
			volumes,
			placement_constraints,
			runtime_platform,
			cpu,
			memory,
			inference_accelerators,
			pid_mode,
			ipc_mode,
			proxy_configuration,
			ephemeral_storage,
		])
		.set_requires_compatibilities(service_task_definition.requires_compatibilities)
		.send()
		.await
		.wrap_err("update task definition")?;

	let new_arn = result
		.task_definition
		.ok_or_else(|| format_err!("no task definition returned after update"))?
		.task_definition_arn
		.ok_or_else(|| format_err!("no task definition ARN returned after update"))?;

	println!("new revision: {new_arn}");

	client
		.update_service()
		.cluster("arn:aws:ecs:eu-north-1:412850343551:cluster/dbost-cluster")
		.service("arn:aws:ecs:eu-north-1:412850343551:service/dbost-cluster/dbost")
		.task_definition(new_arn)
		.send()
		.await
		.wrap_err("update service")?;

	println!("service successfully updated");
	Ok(())
}
