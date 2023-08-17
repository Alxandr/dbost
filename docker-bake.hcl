variable "version" {
	default = "latest"
}

group "default" {
  targets = ["web", "migrator", "db-cleaner", "deployer"]
}

target "_base" {
	dockerfile = "Dockerfile"
	platforms = ["linux/amd64"]
	// platforms = ["linux/amd64", "linux/arm64"]
	cache-from = ["type=gha", "type=gha,scope=main"]
	cache-to = ["type=gha,mode=max"]
}

target "web" {
  inherits = [ "_base" ]
	target = "web"
	tags = [ "ghcr.io/alxandr/dbost", "ghcr.io/alxandr/dbost:${version}" ]
}

target "migrator" {
  inherits = [ "_base" ]
	target = "migrator"
	tags = [ "ghcr.io/alxandr/dbost/migrator", "ghcr.io/alxandr/dbost/migrator:${version}" ]
}

target "deployer" {
  inherits = [ "_base" ]
	target = "deployer"
	args = {
		VERSION = "${version}"
	}
	tags = [ "ghcr.io/alxandr/dbost/deployer", "ghcr.io/alxandr/dbost/deployer:${version}" ]
}

target "db-cleaner" {
  inherits = [ "_base" ]
	target = "db-cleaner"
	tags = [ "ghcr.io/alxandr/dbost/db-cleaner", "ghcr.io/alxandr/dbost/db-cleaner:${version}" ]
}
