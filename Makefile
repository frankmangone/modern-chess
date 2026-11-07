SHELL := /bin/sh

IMAGE ?= modern-chess-server:dev
API_PORT ?= 3001

.PHONY: help api-run api-check api-test api-build docker-build docker-run

help:
	@echo "Targets:"
	@echo "  api-run       Run the API locally (requires DATABASE_URL in env)"
	@echo "  api-check     Cargo check for server crate"
	@echo "  api-test      Run server tests"
	@echo "  api-build     Build server binary in release mode"
	@echo "  docker-build  Build API container image"
	@echo "  docker-run    Run API container (requires DATABASE_URL var)"

api-run:
	cargo run -p modern-chess-server

api-check:
	cargo check -p modern-chess-server

api-test:
	cargo test -p modern-chess-server

api-build:
	cargo build --release -p modern-chess-server

docker-build:
	docker build -t $(IMAGE) .

docker-run:
	@test -n "$(DATABASE_URL)" || (echo "DATABASE_URL is required" && exit 1)
	docker run --rm \
		-p $(API_PORT):$(API_PORT) \
		-e API_HOST=0.0.0.0 \
		-e API_PORT=$(API_PORT) \
		-e DATABASE_URL="$(DATABASE_URL)" \
		-e DB_MIGRATE_ON_STARTUP=true \
		-e DB_SEED_DEFAULT_SPECS=true \
		$(IMAGE)
