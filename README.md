# Coffee Labs — Enterprise Rust Axum Backend v2.0.0

High-performance, ultra-secure, low-memory **Rust (Axum 0.8)** backend serving the **Coffee Labs Flutter Mobile Application** and an integrated **Real-Time APM & API Response Time Observability Dashboard**, backed by **Serverless NeonDB PostgreSQL**.

---

## 🏛️ Core Architecture & Engineering Pillars

1. **Mechanical Sympathy & Low Memory Footprint**
   - **`mimalloc`** (`#[global_allocator]`) for minimal heap fragmentation and extreme multi-core concurrency.
   - **Brotli (`br`) & Gzip** response compression, `bytes`, and lock-free `DashMap` telemetry metrics.
   - **NeonDB Serverless Optimized Pool**: `min_connections = 0` (scale-to-zero support), `max_connections = 15`, `acquire_timeout = 12s` (cold-start resilience), `idle_timeout = 300s`, `max_lifetime = 1800s`, and `test_before_acquire(true)`.

2. **Defense-in-Depth Security (Zero-Trust)**
   - **Argon2id** (`v0x13`) password hashing with transparent migration from legacy hashes.
   - **Asymmetric Ed25519 (`EdDSA`)** JWT signing & verification + **Refresh Token Family & Reuse Detection** (SHA-256 hashed refresh tokens in NeonDB; token reuse immediately revokes the entire device family).
   - **Zero-Trust Payload Validation**: Custom `ValidatedJson<T>` Axum extractor enforcing `validator` rules before reaching any handler.
   - **Rate Limiting**: `tower_governor` + sliding-window IP guard (`429 Too Many Requests`).
   - **Memory-Safe Secrets**: `secrecy::SecretString` wrapping database URLs, passwords, and cryptographic seeds.

3. **Extreme Modularity (< 200 LOC Law)**
   - Every domain module is strictly isolated into `routes.rs`, `handlers.rs`, `service.rs`, `repository.rs`, `dto.rs`, `models.rs`, and `error.rs`.
   - Zero `.unwrap()` or `.expect()` in production pathways; all errors map cleanly into a centralized `AppError` (`thiserror`) without leaking SQL internals.

---

## 📊 Real-Time Backend Observability Dashboard (`/dashboard`)

Open `http://localhost:3000/dashboard` (or `/`) in any browser to view:
- **Live Throughput & Latency Percentiles**: Per-endpoint `Min`, `Avg`, `P50`, `P95`, `P99`, and `Max` response times in milliseconds.
- **NeonDB Serverless Pool Health**: Live DB ping latency (ms), active/idle connection counters, and domain table row counts.
- **HTTP Status & Security Guard Distribution**: Real-time `2xx`, `3xx`, `4xx`, and `5xx` counters + live scrolling request trace stream.
- **1-Click API Benchmark**: Built-in benchmark runner that exercises 8 core mobile endpoints concurrently.
- **OpenAPI 3.1 Specification**: Available at `/api/v1/openapi.json` (powered by `utoipa`).

---

## 📂 Modular Directory Structure

```text
src/
├── main.rs                      # Entrypoint ONLY: mimalloc, tracing, config, server (< 80 LOC)
├── app.rs                       # Router composition & global Tower middleware stack
├── core/                        # Shared infrastructure & cross-cutting concerns
│   ├── config.rs                # Env parsing using `secrecy::SecretString`
│   ├── db.rs                    # NeonDB PgPool setup & serverless tuning
│   ├── migrations.rs            # Idempotent schema & composite index initialization
│   ├── seeds.rs                 # Default Admin, Gatekeeper, Social & Curated Bean seeds
│   ├── error.rs                 # Global `AppError` & sanitized HTTP JSON responses
│   ├── extractors.rs            # `ValidatedJson<T>`, `AuthUser`, `AdminUser`, `RequireTier<T>`
│   ├── middleware.rs            # `tower_governor`, Security Headers, Request ID & APM timer
│   ├── pagination.rs            # Base64 Cursor-based pagination for mobile infinite scroll
│   ├── security.rs              # Argon2id, Ed25519 JWT keyring, SHA-256 Refresh Token hashing
│   └── telemetry.rs             # Lock-free DashMap endpoint latency & percentile registry
└── modules/                     # 10 Isolated Domain Services (< 200 LOC per file)
    ├── auth/                    # Login, Register, Ed25519 JWT, Refresh Token Family Rotation
    ├── rbac/                    # Account Tiers (Trial, Pro, Business), Quotas & Feature Guards
    ├── social/                  # Community Feed, Cursor Pagination, Likes, Nested Comments, Follows
    ├── users/                   # Admin User Approval, Stats & Barista Profile/Equipment Sync
    ├── recipes/                 # Cloud Recipe Sync, WBC/WBrC Custom Recipes & Favorites
    ├── brew_logs/               # Brew History, Sensory Journal, Cupping & Brew Analytics
    ├── beans/                   # Curated Roastery Beans, Bean Radar & AI Barista Dial-In Lab
    ├── marketplace/             # Roastery Marketplace, Voucher Engine & Thermal Orders
    ├── platform/                # Hybrid 3-Tier Version Gatekeeper (Shorebird OTA) & Feedbacks
    └── telemetry/               # Real-Time APM Dashboard UI & OpenAPI 3.1 Specification
```
