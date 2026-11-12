SHELL := /bin/sh

IMAGE ?= modern-chess-game-api:dev
API_PORT ?= 3001

DB_CONTAINER ?= modern-chess-db
DB_PORT ?= 5455
DB_USER ?= modern_chess
DB_PASSWORD ?= modern_chess

.PHONY: help covers-init cover covers-seed storage-up storage-down db-up db-down dev-keys dev share api-run api-check api-test api-build docker-build docker-run

help:
	@echo "Targets:"
	@echo "  storage-up    Start MinIO (S3-compatible image storage) in Docker"
	@echo "  storage-down  Stop MinIO; the stored images are kept"
	@echo "  covers-init   Create the image bucket and open its covers/ folder (development)"
	@echo "  cover         Set a game cover: make cover GAME=chess FILE=path/to/image.png"
	@echo "  covers-seed   Set every game cover from assets/covers/<game>.jpg (development)"
	@echo "  db-up         Start Postgres 16 in Docker with the game and social databases"
	@echo "  db-down       Stop and remove that container and its data"
	@echo "  dev-keys      Write development .env files with a fresh signing key pair"
	@echo "  dev           Start the database, then game-api (3001) and social-api (3002)"
	@echo "  share         Run everything and open a temporary public address for friends (see deploy/README.md)"
	@echo "  api-run       Run the API locally (requires DATABASE_URL in env)"
	@echo "  api-check     Cargo check for server crate"
	@echo "  api-test      Run server tests"
	@echo "  test-damage   Damage every value of every game spec and play it (about 3 minutes)"
	@echo "  api-build     Build server binary in release mode"
	@echo "  docker-build  Build API container image"
	@echo "  docker-run    Run API container (requires DATABASE_URL and AUTH_PUBLIC_KEY_PEM vars)"

db-up:
	@docker start $(DB_CONTAINER) >/dev/null 2>&1 || docker run -d --name $(DB_CONTAINER) \
		-e POSTGRES_USER=$(DB_USER) -e POSTGRES_PASSWORD=$(DB_PASSWORD) \
		-p 127.0.0.1:$(DB_PORT):5432 postgres:16-alpine >/dev/null
	@until docker exec $(DB_CONTAINER) pg_isready -U $(DB_USER) >/dev/null 2>&1; do sleep 0.5; done
	@for db in game social; do \
		docker exec $(DB_CONTAINER) psql -U $(DB_USER) -d postgres -tAc \
			"SELECT 1 FROM pg_database WHERE datname='$$db'" | grep -q 1 \
			|| docker exec $(DB_CONTAINER) createdb -U $(DB_USER) $$db; \
	done
	@echo "Postgres is up on localhost:$(DB_PORT) with databases: game, social"

db-down:
	-docker rm -f $(DB_CONTAINER)

storage-up:
	docker compose up -d --wait minio
	@echo "MinIO is up: API on localhost:9000, console on http://localhost:9001"

storage-down:
	docker compose down

# The same throwaway credentials the compose file gives MinIO. Only these targets get them:
# the web server never holds the means to write images.
IMAGES_ACCESS_KEY ?= modern_chess
IMAGES_SECRET_KEY ?= modern_chess_secret
COVERS = IMAGES_ACCESS_KEY=$(IMAGES_ACCESS_KEY) IMAGES_SECRET_KEY=$(IMAGES_SECRET_KEY) \
	cargo run -q -p modern-chess-game-api --bin covers --

covers-init:
	$(COVERS) init

cover:
	@test -n "$(GAME)" -a -n "$(FILE)" || (echo "usage: make cover GAME=chess FILE=path/to/image.png" && exit 1)
	$(COVERS) set $(GAME) $(FILE)

# One cover per file in assets/covers, named after the game's slug.
covers-seed: covers-init
	@for file in assets/covers/*.jpg; do \
		$(COVERS) set $$(basename $$file .jpg) $$file || exit 1; \
	done

dev-keys:
	cargo run -q -p modern-chess-testkeys --example dev_keys

dev: db-up dev-keys
	@trap 'kill 0' INT TERM; \
	cargo run -q -p modern-chess-social-api & \
	cargo run -q -p modern-chess-game-api & \
	wait

# Stop make dev first: both use ports 3001 and 3002.
share: db-up storage-up dev-keys
	pnpm --dir apps/web build
	sh scripts/share.sh

api-run:
	cargo run -p modern-chess-game-api

api-check:
	cargo check -p modern-chess-game-api

api-test:
	cargo test -p modern-chess-game-api

api-build:
	cargo build --release -p modern-chess-game-api

docker-build:
	docker build -t $(IMAGE) .

docker-run:
	@test -n "$(DATABASE_URL)" || (echo "DATABASE_URL is required" && exit 1)
	@test -n "$(AUTH_PUBLIC_KEY_PEM)" || (echo "AUTH_PUBLIC_KEY_PEM is required" && exit 1)
	docker run --rm \
		-p $(API_PORT):$(API_PORT) \
		-e API_HOST=0.0.0.0 \
		-e API_PORT=$(API_PORT) \
		-e DATABASE_URL="$(DATABASE_URL)" \
		-e AUTH_PUBLIC_KEY_PEM="$(AUTH_PUBLIC_KEY_PEM)" \
		-e DATABASE_MIGRATE_ON_STARTUP=true \
		-e DATABASE_SEED_DEFAULT_SPECS=true \
		$(IMAGE)

# Slow, so not part of `cargo test`: changes one value at a time in every shipped spec and
# requires that loading it and playing a few moves never panics.
.PHONY: test-damage
test-damage:
	cargo test -p modern-chess-session --test malformed_specs -- --include-ignored
