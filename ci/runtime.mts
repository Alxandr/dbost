import type { Container } from "@dagger.io/dagger";

export const runtime = (container: Container): Container =>
	container
		.from("docker.io/debian:bookworm-slim")
		.withExec([
			"sh",
			"-c",
			"apt-get update && apt-get install -y curl tini && rm -rf /var/lib/apt/lists/*",
		]);
