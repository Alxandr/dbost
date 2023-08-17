variable "version" {
	default = "latest"
}

group "default" {
  targets = ["web", "migrator", "db-cleaner"]
}

target "_base" {
	dockerfile = "Dockerfile"
	platforms = ["linux/amd64"]
	// platforms = ["linux/amd64", "linux/arm64"]
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

target "db-cleaner" {
  inherits = [ "_base" ]
	target = "db-cleaner"
	tags = [ "ghcr.io/alxandr/dbost/db-cleaner", "ghcr.io/alxandr/dbost/db-cleaner:${version}" ]
}
