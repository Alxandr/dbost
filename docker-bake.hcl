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
	tags = [ "ghcr.io/alxandr/dbost" ]
}

target "migrator" {
  inherits = [ "_base" ]
	target = "job"
	args =  {
		BIN_NAME = "dbost-migration"
		PACKAGE = "dbost-migration"
	}
	tags = [ "ghcr.io/alxandr/dbost/migrator" ]
}

target "db-cleaner" {
  inherits = [ "_base" ]
	target = "job"
	args =  {
		BIN_NAME = "dbost-jobs-db-cleanup"
		PACKAGE = "dbost-jobs-db-cleanup"
	}
	tags = [ "ghcr.io/alxandr/dbost/db-cleaner" ]
}
