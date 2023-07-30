[private]
@list:
	just --list

# start dev server
run:
	cargo shuttle run

# deploy to shuttle
deploy:
	cargo shuttle deploy

[private]
alias start := run

# generate entities from database schema
generate-entities:
	sea-orm-cli generate entity -o db/entities/src -l

# run migrations
migrate cmd:
	sea-orm-cli migrate -d db/migrations {{cmd}}
