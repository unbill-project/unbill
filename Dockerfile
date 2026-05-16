# ────────────────────────────────────────────────────────────────────────────
# Stage 1 — build unbill-server (static musl binary for Alpine compat)
# ────────────────────────────────────────────────────────────────────────────
# sirno:witness:deployment-topologies:begin
FROM rust:1-slim-bookworm AS server-builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    musl-tools pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl --bin unbill-server

# ────────────────────────────────────────────────────────────────────────────
# Stage 2 — build unbill-ui-remote (WASM + static assets via Trunk)
# ────────────────────────────────────────────────────────────────────────────
FROM rust:1-slim-bookworm AS frontend-builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
    https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
    | bash
RUN cargo binstall -y trunk

COPY . .

WORKDIR /app/apps/unbill-ui-remote
RUN trunk build --release

# ────────────────────────────────────────────────────────────────────────────
# Stage 3 — runtime: Caddy serves static files, proxies /api/v1 to the server
# ────────────────────────────────────────────────────────────────────────────
FROM caddy:2 AS runtime

COPY --from=server-builder \
    /app/target/x86_64-unknown-linux-musl/release/unbill-server \
    /usr/local/bin/unbill-server

COPY --from=frontend-builder \
    /app/apps/unbill-ui-remote/dist \
    /app/static

COPY docker/Caddyfile     /etc/caddy/Caddyfile
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Required:  API_KEY         — bearer token enforced by unbill-server
# Optional:  PORT            — internal server port (default 8080)
# Optional:  UNBILL_DATA_DIR — override ledger storage path (mount a volume here)
ENV UNBILL_DATA_DIR=/data

VOLUME ["/data"]
EXPOSE 80

ENTRYPOINT ["/entrypoint.sh"]
# sirno:witness:deployment-topologies:end
