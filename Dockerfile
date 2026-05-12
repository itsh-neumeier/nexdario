FROM rust:1.77-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev sqlite3 && rm -rf /var/lib/apt/lists/*

RUN cargo install sqlx-cli --no-default-features --features sqlite

WORKDIR /app

COPY Cargo.toml ./
COPY migrations ./migrations
COPY src ./src

# Create a temp database, run migrations, prepare sqlx offline metadata, then build
RUN export DATABASE_URL="sqlite:///tmp/build.db" && \
    sqlx database create && \
    sqlx migrate run --source migrations && \
    cargo sqlx prepare && \
    cargo build --release

# ─── Runtime ────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/nexdario /app/nexdario
COPY templates ./templates
COPY static ./static

RUN mkdir -p /data/backups /data/exports

ENV DATABASE_URL=sqlite:///data/nexdario.db \
    BIND_HOST=0.0.0.0 \
    BIND_PORT=8080 \
    DATA_DIR=/data \
    BACKUP_DIR=/data/backups \
    EXPORT_DIR=/data/exports \
    TEMPLATES_DIR=templates \
    STATIC_DIR=static

VOLUME ["/data"]
EXPOSE 8080

CMD ["/app/nexdario"]
