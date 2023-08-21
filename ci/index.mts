import { Client, connect } from "@dagger.io/dagger";
import { getSccache } from "./sccache.mjs";
import { runtime as runtimeContainer } from "./runtime.mjs";

const PUBLISH = process.env.PUBLISH === "true";
const VERSION = process.env.VERSION || "latest";

const DB_CLEANER = "dbost-jobs-db-cleanup";
const PRECOMPRESS = "dbost-jobs-precompress";
const MIGRATION = "dbost-migration";
const DBOST = "dbost";
const DEPLOYER = "dbost-jobs-deploy";
const executables = [
	DB_CLEANER,
	PRECOMPRESS,
	MIGRATION,
	DBOST,
	DEPLOYER,
] as const;

// initialize Dagger client
await connect(
	async (client: Client) => {
		const sccache = getSccache(client);
		const pnpmCache = client.cacheVolume("pnpm");
		const targetCache = client.cacheVolume("target");

		const sources = client.host().directory(".", {
			exclude: ["target", "node_modules"],
		});

		const chef = client
			.pipeline("prepare")
			.container()
			.from("docker.io/lukemathwalker/cargo-chef:latest-rust-slim-bookworm")
			.withExec([
				"sh",
				"-c",
				"apt-get update && apt-get install -y curl ca-certificates clang && rm -rf /var/lib/apt/lists/*",
			])
			.withEnvVariable("CARGO_TERM_COLOR", "always")
			.withWorkdir("/app")
			.with(sccache.install);

		const recipe = chef
			.withDirectory(".", sources, {
				include: [
					"**/Cargo.toml",
					"Cargo.lock",
					"**/main.rs",
					"**/lib.rs",
					"**/build.rs",
				],
			})
			.withExec(["cargo", "chef", "prepare", "--recipe-path", "recipe.json"])
			.file("recipe.json");

		const builder = chef
			.pipeline("build")
			.withFile("recipe.json", recipe)
			.withMountedCache("target", targetCache)
			.withExec(["sh", "-c", "echo $RUSTC_WRAPPER"])
			.withExec([
				"cargo",
				"chef",
				"cook",
				"--release",
				"--workspace",
				"--recipe-path",
				"recipe.json",
			])
			.withDirectory(".", sources, {
				include: ["**/Cargo.toml", "Cargo.lock", "**/*.rs"],
			})
			.withEnvVariable("GIT_SHA", VERSION)
			.withExec(["cargo", "build", "--release", "--workspace"])
			.withExec(["mkdir", "-p", "out"])
			.withExec([
				"cp",
				...executables.map((name) => `target/release/${name}`),
				"out/",
			]);

		const test = builder
			.pipeline("test")
			.withExec(["cargo", "test", "--workspace", "--release"]);

		const clippy = test
			.pipeline("clippy")
			.withExec(["rustup", "component", "add", "clippy"])
			.withExec([
				"cargo",
				"clippy",
				"--workspace",
				"--release",
				"--",
				"-D",
				"warnings",
			]);

		const clippyOutput = await clippy.stdout();
		const testOutput = await test.stdout();
		const sccacheStats = await sccache.stats(clippy);

		const bins = {
			dbost: builder.file(`out/${DBOST}`),
			precompress: builder.file(`out/${PRECOMPRESS}`),
			deployer: builder.file(`out/${DEPLOYER}`),
			migrator: builder.file(`out/${MIGRATION}`),
			dbCleaner: builder.file(`out/${DB_CLEANER}`),
		};

		const assets = client
			.pipeline("client")
			.container()
			.from("docker.io/node:lts")
			.withWorkdir("/app")
			.withEnvVariable("PNPM_HOME", "/pnpm")
			.withEnvVariable("npm_config_package_import_method", "copy")
			.withEnvVariable("PATH", "$PNPM_HOME:$PATH", { expand: true })
			.withExec(["corepack", "enable"], { skipEntrypoint: true })
			.withMountedCache("/pnpm/store", pnpmCache)
			.withDirectory(".", sources, {
				include: ["package.json", "pnpm-lock.yaml"],
			})
			.withExec(["pnpm", "install", "--frozen-lockfile"])
			.withDirectory(".", sources)
			.withExec(["pnpm", "build"])
			.withFile("/usr/local/bin/dbost-jobs-precompress", bins.precompress)
			.withExec(["/usr/local/bin/dbost-jobs-precompress", "--dir", "/app/dist"])
			.directory("dist");

		const runtime = client
			.pipeline("runtime")
			.container()
			.with(runtimeContainer)
			.withEntrypoint(["tini", "--"]);

		const deployer = runtime
			.pipeline("deployer")
			.withEnvVariable("TAG", VERSION)
			.withFile(`/usr/local/bin/${DEPLOYER}`, bins.deployer)
			.withDefaultArgs({
				args: [`/usr/local/bin/${DEPLOYER}`],
			});

		const migrator = runtime
			.pipeline("migrator")
			.withFile(`/usr/local/bin/${MIGRATION}`, bins.migrator)
			.withDefaultArgs({
				args: [`/usr/local/bin/${MIGRATION}`],
			});

		const dbCleaner = runtime
			.pipeline("db-cleaner")
			.withFile(`/usr/local/bin/${DB_CLEANER}`, bins.dbCleaner)
			.withDefaultArgs({
				args: [`/usr/local/bin/${DB_CLEANER}`],
			});

		const web = runtime
			.pipeline("web")
			.withFile(`/usr/local/bin/${DBOST}`, bins.dbost)
			.withDirectory("/var/www/public", assets)
			.withEnvVariable("WEB_PUBLIC_PATH", "/var/www/public")
			.withExposedPort(8000)
			.withDefaultArgs({
				args: [`/usr/local/bin/${DBOST}`],
			});

		const tags = new Set([VERSION, "latest"]);
		const images = {
			"ghcr.io/alxandr/dbost": web,
			"ghcr.io/alxandr/dbost/migrator": migrator,
			"ghcr.io/alxandr/dbost/deployer": deployer,
			"ghcr.io/alxandr/dbost/db-cleaner": dbCleaner,
		};

		if (PUBLISH) {
			for (const tag of tags) {
				for (const [name, container] of Object.entries(images)) {
					const published = await container.publish(`${name}:${tag}`);
					console.log(`Published ${published}`);
				}
			}
		} else {
			console.log(`Skipping publish as $PUBLISH is not set to true`);
			for (const container of Object.values(images)) {
				await container.sync();
			}
		}

		console.log(`Clippy output: ${clippyOutput}`);
		console.log(`Test output: ${testOutput}`);
		if (sccacheStats) console.log(`Sccache stats: ${sccacheStats}`);
	},
	{ LogOutput: process.stdout }
);
