# ==============================================================================
# Stage 1: Rust Builder (Supports edition2024 crates via latest stable Cargo)
# ==============================================================================
FROM rust:1-bookworm AS builder

WORKDIR /app

# Pastikan toolchain stable terbaru aktif (mendukung Rust edition2024 / Cargo >= 1.85+)
RUN rustup update stable && rustup default stable && cargo --version && rustc --version

# Copy manifest dan file compile-time (include_str! seeds & admin-panel)
COPY Cargo.toml Cargo.lock ./
COPY admin-panel ./admin-panel
COPY storage/*.json ./storage/
COPY src ./src

# Build binary release utama (LTO & mimalloc aktif)
RUN cargo build --release --bin coffe-labs-backend

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
