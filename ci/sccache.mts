import type { Client, Container } from "@dagger.io/dagger";
import { Octokit } from "@octokit/rest";
import { runtime } from "./runtime.mjs";

const octokit = new Octokit();
const latestRelease = await octokit.repos.getLatestRelease({
	owner: "mozilla",
	repo: "sccache",
});

const sccacheVersion = latestRelease.data.tag_name;
const stemName = `sccache-${sccacheVersion}-x86_64-unknown-linux-musl`;
const fileName = `${stemName}.tar.gz`;
const sccacheArtifact =
	latestRelease.data.assets.find(
		(asset) =>
			asset.name ===
			`sccache-${sccacheVersion}-x86_64-unknown-linux-musl.tar.gz`
	)?.browser_download_url ?? null;
const sccacheSha256Artifact =
	latestRelease.data.assets.find(
		(asset) =>
			asset.name ===
			`sccache-${sccacheVersion}-x86_64-unknown-linux-musl.tar.gz.sha256`
	)?.browser_download_url ?? null;

const sccacheConfig = (binLocation: string) => {
	if (sccacheArtifact === null) {
		console.warn(`sccache artifact not found`);
		return null;
	}

	const bucket = process.env.SCCACHE_BUCKET;
	const region = process.env.SCCACHE_REGION;
	const endpoint = process.env.SCCACHE_ENDPOINT;
	const accessKeyId = process.env.SCCACHE_ACCESS_KEY_ID;
	const secretAccessKey = process.env.SCCACHE_SECRET_ACCESS_KEY;

	if (!endpoint || !accessKeyId) {
		return null;
	}

	return {
		RUSTC_WRAPPER: binLocation,
		SCCACHE_PATH: binLocation,
		CARGO_INCREMENTAL: "0",
		SCCACHE_BUCKET: bucket,
		SCCACHE_REGION: region,
		SCCACHE_ENDPOINT: endpoint,
		AWS_ACCESS_KEY_ID: accessKeyId,
		AWS_SECRET_ACCESS_KEY: secretAccessKey,
	};
};

const env =
	(vars: Record<string, string | undefined>) =>
	(container: Container): Container => {
		for (const [key, value] of Object.entries(vars)) {
			if (value) {
				container = container.withEnvVariable(key, value);
			}
		}

		return container;
	};

export type Sccache = {
	install: (container: Container) => Container;
	stats: (container: Container) => Promise<string | null>;
};

export const getSccache = (client: Client): Sccache => {
	const BIN_LOCATION = "/usr/local/bin/sccache";
	const config = sccacheConfig(BIN_LOCATION);
	if (!config) {
		console.log(`sccache not configured`);

		return Object.freeze({
			install: (container: Container) => container,
			stats: async (_container: Container) => null,
		});
	}

	const sccacheArchive = client.http(sccacheArtifact!);
	const sccacheSha = client.http(sccacheSha256Artifact!);

	const extractContainer = client
		.container()
		.with(runtime)
		.withWorkdir("/sccache")
		.withFile(fileName, sccacheArchive)
		.withFile(`${fileName}.sha256`, sccacheSha);

	const sccacheBinary = extractContainer
		.withExec(["sh", "-c", `echo " ${fileName}" >> ${fileName}.sha256`])
		.withExec(["sh", "-c", `sha256sum -c ${fileName}.sha256`])
		.withExec(["sh", "-c", `tar -xzf ${fileName}`])
		.file(`${stemName}/sccache`);

	const install = (container: Container): Container => {
		return container.with(env(config)).withFile(BIN_LOCATION, sccacheBinary);
	};

	const stats = async (container: Container) => {
		return container.withExec([BIN_LOCATION, "--show-stats"]).stdout();
	};

	return Object.freeze({
		install,
		stats,
	});
};
