SHELL := /bin/sh

IMAGE ?= modern-chess-game-api:dev
API_PORT ?= 3001

DB_CONTAINER ?= modern-chess-db
DB_PORT ?= 5455
DB_USER ?= modern_chess
DB_PASSWORD ?= modern_chess

.PHONY: help web-deps lint-panics covers-init cover covers-seed storage-up storage-down db-up db-down dev-keys wasm book dev share api-run api-check api-test api-build docker-build docker-run

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
	@echo "  wasm          Rebuild the engine's WebAssembly build that the web app and the docs load"
	@echo "  book          Build the docs book into apps/docs/book (served at /docs/ by dev and share)"
	@echo "  dev           Rebuild the wasm and start the database, then game-api (3001), social-api (3002) and the web app (3050)"
	@echo "  share         Run everything and open a temporary public address for friends (see deploy/README.md)"
	@echo "  api-run       Run the API locally (requires DATABASE_URL in env)"
	@echo "  lint-panics   Fail on unwrap/expect in server code (tests and build scripts are exempt)"
	@echo "  api-check     Cargo check for server crate"
	@echo "  api-test      Run server tests"
	@echo "  test-damage   Damage every value of every game spec and play it (about 3 minutes)"
	@echo "  api-build     Build server binary in release mode"
	@echo "  docker-build  Build API container image"
	@echo "  docker-run    Run API container (requires DATABASE_URL and AUTH_PUBLIC_KEY_PEM vars)"

# Lib and bin targets only, so tests may still unwrap. A deliberate exception needs an
# `#[allow(clippy::expect_used, reason = "...")]` that says why it cannot happen.
lint-panics:
	cargo clippy --no-deps -p modern-chess-game -p modern-chess-social -p modern-chess-kernel \
		-p modern-chess-game-api -p modern-chess-social-api \
		-- -D clippy::unwrap_used -D clippy::expect_used

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

# The web app and the docs book load the engine from apps/docs/theme/pkg (git-ignored). A stale
# build refuses specs the servers accept, so `make dev` rebuilds it (and runs vite directly, since
# `pnpm dev` would rebuild it again); reload the page afterwards.
wasm:
	apps/docs/build-engine.sh

# The docs book, served at /docs/ by the dev server and by the share gateway. It needs mdbook; a
# machine without it still runs the app, and /docs/ says the book is not built. The engine is
# already built by `wasm`, so the book skips it.
book:
	@if command -v mdbook >/dev/null 2>&1; then SKIP_ENGINE=1 apps/docs/build.sh; \
	else echo "mdbook is not installed, so /docs/ will be empty (brew install mdbook)"; fi

# Quick when nothing changed; a fresh checkout has no node_modules and vite would not start.
web-deps:
	pnpm --dir apps/web install --frozen-lockfile

# The web app starts once both APIs answer /health: started together, vite is up long before cargo
# has built, migrated and seeded, and its proxy refuses the page's first requests. If an API exits
# before it is ready, everything stops rather than waiting forever.
dev: wasm book web-deps db-up dev-keys
	@echo "Starting game-api on :3001 and social-api on :3002"
	@trap 'kill 0' INT TERM; \
	cargo run -q -p modern-chess-social-api & social=$$!; \
	cargo run -q -p modern-chess-game-api & game=$$!; \
	for port in 3001 3002; do \
	  until curl -sf -o /dev/null http://localhost:$$port/health; do \
	    if ! kill -0 $$social 2>/dev/null || ! kill -0 $$game 2>/dev/null; then \
	      echo "An API exited before it was ready; stopping."; kill 0; exit 1; \
	    fi; \
	    sleep 1; \
	  done; \
	done; \
	echo "APIs ready. Starting the web app on http://localhost:3050 (the book is at /docs/)"; \
	pnpm --dir apps/web exec vite --port 3050 --strictPort & \
	wait

# Stop make dev first: both use ports 3001 and 3002.
share: wasm book db-up storage-up dev-keys
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
