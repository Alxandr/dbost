[private]
@list:
	just --list

# start dev server
run: build-css
	cargo shuttle run

# build for production
build: build-css
	cargo build --release

# build css
build-css:
	pnpm run build

# deploy to shuttle
deploy: build
	cargo shuttle deploy

[private]
alias start := run

# generate entities from database schema
generate-entities:
	sea-orm-cli generate entity -o db/entities/src -l

# run migrations
migrate +cmd:
	sea-orm-cli migrate -d db/migrations {{cmd}}
