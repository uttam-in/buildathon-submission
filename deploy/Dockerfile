# syntax=docker/dockerfile:1.7
#
# Multi-stage build for the Digital Menu Board (Axum server).
#
# Build context is the submission root (the dir containing Cargo.toml,
# crates/, migrations/, and the vendored Resources/), e.g.:
#
#   docker build -f deploy/Dockerfile -t dmbr .
#
# Cloud Build (cloudbuild.yaml) and deploy.sh both set the context correctly.

# ---- Stage 1: build the Rust workspace ----------------------------------
# 1.88+ is required: the locked transitive dep home 0.5.12 declares
# `rust-version = 1.88` (and uses edition2024, stable since 1.85).
FROM rust:1.88-slim-bookworm AS builder

# pkg-config / libssl are not required: sqlx uses rustls (tls-rustls) and there
# is no native-tls dependency in the tree. We link glibc and run on slim debian.
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations

# Build the server and the migration runner in release mode.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release \
        --bin dmbr-server-axum \
        --bin dmbr-migrate \
 && mkdir -p /out \
 && cp target/release/dmbr-server-axum /out/dmbr-server-axum \
 && cp target/release/dmbr-migrate     /out/dmbr-migrate

# ---- Stage 2: minimal runtime -------------------------------------------
FROM debian:bookworm-slim AS runtime

# ca-certificates is required for the rustls TLS handshake to the managed
# Postgres (Railway) over the public internet.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --no-create-home appuser

WORKDIR /app

# Application binaries.
COPY --from=builder /out/dmbr-server-axum /usr/local/bin/dmbr-server-axum
COPY --from=builder /out/dmbr-migrate     /usr/local/bin/dmbr-migrate

# Bake the static menu-board assets (configs/states/menu.json/samples) into
# the image so the server is self-contained. RESOURCES_DIR points here.
COPY Resources /app/Resources
ENV RESOURCES_DIR=/app/Resources
ENV MENU_JSON=/app/Resources/menu.json

# Cloud Run injects PORT; the server reads it and binds 0.0.0.0:$PORT.
ENV PORT=8080
EXPOSE 8080

USER appuser

CMD ["dmbr-server-axum"]
