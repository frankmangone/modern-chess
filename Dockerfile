FROM rust:slim AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        pkg-config \
        libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps ./apps

RUN cargo build --release -p modern-chess-game-api

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/modern-chess-game-api /usr/local/bin/modern-chess-game-api

EXPOSE 3001

ENV API_HOST=0.0.0.0 \
    API_PORT=3001 \
    RUST_LOG=modern_chess_game_api=info,info \
    DATABASE_MIGRATE_ON_STARTUP=true \
    DATABASE_SEED_DEFAULT_SPECS=true

ENTRYPOINT ["modern-chess-game-api"]
