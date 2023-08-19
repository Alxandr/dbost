set dotenv-load

entities-proj   := "domain/entities"
migrations-proj := "domain/migrations"

[private]
@list:
	just --list

# build everything on depot
bake:
	sudo pnpm exec tsx ci/index.mts

# start dev server
run: build-assets
	cargo run --features dev

# seed database
seed:
	cargo run --package dbost-jobs-seed

# build for production
build: build-assets
	cargo build --release

# start, and re-start on changes
watch:
	cargo watch -s "just run"

# build css
build-assets:
	pnpm run build
	RUST_LOG=warn cargo run -p dbost-jobs-precompress -- --dir dist

# deploy to shuttle
deploy: build
	cargo shuttle deploy

[private]
alias start := run

[private]
alias publish := deploy

# generate entities from database schema
generate-entities:
	sea-orm-cli generate entity -o {{entities-proj}}/src -l --expanded-format --date-time-crate time

# run migrations
migrate +cmd:
	sea-orm-cli migrate -d {{migrations-proj}} {{cmd}}

# build image using podman
podman-build:
	podman build . -t localhost/alxandr/dbost

# run image using podman
podman-run:
	podman run --rm -it -p 8000:8000 --env-host --env WEB_PUBLIC_PATH=/var/www/public localhost/alxandr/dbost
