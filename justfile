set dotenv-load

entities-proj   := "domain/entities"
migrations-proj := "domain/migrations"

[private]
@list:
	just --list

# build everything on depot
bake:
	depot bake

# start dev server
run: build-css
	cargo run --features live-reload

# seed database
seed:
	cargo run --package dbost-jobs-seed

# build for production
build: build-css
	cargo build --release

# start, and re-start on changes
watch:
	cargo watch -s "just run"

# build css
build-css:
	pnpm run build

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
