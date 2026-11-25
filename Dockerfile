# ==============================================================================
# Stage 1: Rust Builder with Dependency Layer Cache (Fast Railway Deploys)
# ==============================================================================
FROM rust:1-bookworm AS builder

WORKDIR /app

# Pre-compile dependency crates in a cached Docker layer so code-only changes
# rebuild in ~30-60s instead of recompiling 300+ crates from scratch.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && echo "" > src/lib.rs \
    && cargo build --release --bin coffe-labs-backend \
    && rm -rf src

# Copy compile-time assets (include_str! seeds & admin-panel) and actual source
COPY admin-panel ./admin-panel
COPY storage/*.json ./storage/
COPY src ./src

# Touch entrypoints so Cargo recompiles the application crate cleanly
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --bin coffe-labs-backend

# ==============================================================================
# Stage 2: Minimal Debian Runtime
# ==============================================================================
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/coffe-labs-backend /usr/local/bin/coffe-labs-backend
COPY storage ./storage
COPY admin-panel ./admin-panel

ENV PORT=3000
ENV ENVIRONMENT=production
ENV RUST_LOG=info,coffe_labs_backend=info

EXPOSE 3000

CMD ["/usr/local/bin/coffe-labs-backend"]
