entities-proj   := "domain/entities"
migrations-proj := "domain/migrations"

[private]
@list:
	just --list

# start dev server
run: build-css
	RUST_LOG="INFO,dbost_session=DEBUG,dbost=DEBUG,sqlx=DEBUG" cargo shuttle run

# build for production
build: build-css
	cargo build --release

# start, and re-start on changes
watch:
	LIVE_RELOAD="true" cargo watch -s "just start"

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
	sea-orm-cli generate entity -o {{entities-proj}}/src -l --expanded-format --date-time-crate time

# run migrations
migrate +cmd:
	sea-orm-cli migrate -d {{migrations-proj}} {{cmd}}
