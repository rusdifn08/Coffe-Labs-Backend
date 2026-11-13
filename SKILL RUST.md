# SKILL RUST.md — Enterprise Rust Axum Backend Design System & Architecture

## 1. Core Identity & Philosophy
This repository is a high-performance, ultra-secure, low-memory Rust (Axum 0.8) backend serving a Flutter Mobile Application, powered by **NeonDB Serverless PostgreSQL**, **Redis/Valkey**, and **CloudAMQP (RabbitMQ)**.
Every line of code must adhere to three pillars:
1. **Extreme Modularity:** Small, single-responsibility files (Max **200 LOC** per `.rs` file).
2. **Defense-in-Depth Security:** Zero-trust input validation, memory-safe secrets, pure-Rust TLS, strict RBAC, and anti-abuse rate limiting.
3. **Mechanical Sympathy & Free-Tier Resilience:** Minimal heap allocations, zero-copy parsing where possible, bounded connection pools, and graceful degradation under **Render Free Tier (512 MB RAM, Native Rust Runtime)** & **CloudAMQP Little Lemur (Max 20 connections)** constraints.

---

## 2. Mandatory Directory Structure
Never dump business logic into `main.rs` or monolithic files. Every domain feature must follow this modular blueprint:

```text
src/
├── main.rs                      # Entrypoint ONLY: initializes mimalloc, tracing, config, workers & boots server
├── app.rs                       # Router composition, AppState wiring & global Tower middleware stack
├── lib.rs                       # Library exports for integration testing
├── core/                        # Shared infrastructure & cross-cutting concerns
│   ├── mod.rs                   # `AppState` + `FromRef` implementations
│   ├── config.rs                # Env parsing using `secrecy::SecretString`
│   ├── db.rs                    # NeonDB PgPool setup, health check, & retry logic
│   ├── rabbitmq/                # Resilient AMQP (`lapin` + rustls): `client.rs`, `publisher.rs`, `topology.rs`, `worker.rs`
│   ├── error.rs                 # Global `AppError` & sanitized HTTP JSON responses
│   ├── extractors.rs            # Custom extractors: `ValidatedJson<T>`, `AuthUser`, `AdminUser`, `RequireTier`
│   ├── middleware.rs            # Rate limiting (`tower_governor`), CORS, Security Headers, Request ID
│   ├── pagination.rs            # Cursor-based pagination structs for mobile infinite scroll
│   └── security.rs              # Argon2id hashing, Ed25519 JWT signing, Refresh Token family logic
└── modules/                     # Domain Services (Strictly Isolated)
    ├── mod.rs
    ├── auth/                    # Login, Register, Token Rotation, Device Sessions
    ├── rbac/                    # Account Tiers (Trial, Pro, Business), Quotas, Feature Flags
    ├── notifications/           # Omnichannel Push Engine (Inbox, FCM v1 HTTP/2, Coalescing, DLQ)
    ├── social_feed/             # Public Profiles, CQRS Projection, Activity Thread Feed
    ├── social_graph/            # Asymmetric Follow/Follback, Mutuals, Privacy Requests
    └── chat/                    # Real-Time WebSocket & MessagePack Hub
```

Within each complex domain module, separate responsibilities strictly:
- `routes.rs` — Axum router definitions (`/api/v1/...`)
- `handlers/` or `handlers.rs` — HTTP/WS transport ONLY (extract validated DTO, delegate, return `ApiResponse`)
- `services/` or `service.rs` — Business rules, caching, state transitions, broker orchestration
- `repositories/` or `repository.rs` — Pure SQLx queries (zero HTTP types or transport concerns)
- `dto.rs` & `models.rs` & `error.rs` — Contracts, DB projections, and domain error variants

---

## 3. Modern Rust Idioms, Safety & Error Architecture
1. **Zero Panics in Production (`src/`):** `.unwrap()`, `.expect()`, `panic!()`, `todo!()`, and unchecked indexing are FORBIDDEN in production runtime paths (`tests/` and `#[cfg(test)]` excepted). Propagate errors cleanly with `?`, `.ok_or_else(...)`, or explicit fallback handling.
2. **Typed Error Hierarchies:** Define domain errors via `thiserror::Error` and convert explicitly to `AppError` so internal SQL/broker details stay inside server logs while clients receive sanitized, deterministic JSON (`{ success: false, error_code, message }`).
3. **Borrow Over Clone & Explicit Ownership:** Prefer `&str` / `&[T]` on internal helpers instead of cloning `String` or `Vec`; reserve `Arc<T>` for shared immutable state or concurrent handles across tasks.
4. **Strict Module File Ceiling (< 200 LOC):** Split large modules proactively into focused submodules (`handlers/`, `services/`, `repositories/`) with clean re-exports rather than exceeding 200 lines in a single file.
5. **Clippy & Compiler Cleanliness:** Code must compile cleanly without warnings under `cargo check` and `cargo clippy`; fix unused imports, dead variables, and future-compatibility lints immediately.

---

## 4. Async Runtime, Concurrency & Memory Discipline (`tokio`)
1. **Non-Blocking Event Loop:** Never run blocking filesystem, CPU-heavy image processing, or synchronous network calls directly on Tokio async worker threads; offload heavy blocking work with `tokio::task::spawn_blocking`.
2. **Bounded Buffers & Backpressure:** Avoid unbounded memory growth in long-running workers (`moka` caches require explicit `.max_capacity(...)` + TTL; batch DB queries with `.clamp(1, 50)`; AMQP consumers MUST set `basic_qos` prefetch limits).
3. **Lock & `.await` Safety:** Never hold `std::sync::Mutex` or `RwLock` guards across `.await` points; use short critical sections, `DashMap`, or lock-free atomics (`Ordering::Relaxed`/`AcqRel`).
4. **Resilient Background Workers:** Always supervise spawned workers (`tokio::spawn`) with internal reconnect loops and bounded exponential backoff + jitter so transient network failures recover cleanly without crashing the Axum HTTP server.

---

## 5. Secrets, Cryptography & Pure-Rust TLS Standards
1. **Secret Hygiene (`secrecy::SecretString`):** Wrap `DATABASE_URL`, `JWT_SECRET_SEED`, `RABBITMQ_URL`, and third-party API credentials in `SecretString`. NEVER print or log connection strings verbatim (strip `user:pass@` before emitting any diagnostic `tracing` event).
2. **Pure-Rust `rustls` Everywhere:** Standardize TLS across `sqlx` (`tls-rustls`), `reqwest` (`rustls-tls`), `fred` (`enable-rustls`), and `lapin` (`rustls`) to guarantee reproducible Native Rust builds on Linux/Render and Windows without OpenSSL system library mismatches.
3. **Zero-Trust Validation:** Extract payloads using `ValidatedJson<T>` (`validator`/`garde`) and bind all SQL parameters safely using SQLx `.bind(...)` (never format untrusted strings into SQL).

---

## 6. Messaging, CloudAMQP (RabbitMQ) & Free-Tier Deployment Rules
1. **Native Rust Runtime (No Docker):** Build & deploy directly via Render's Native Rust toolchain:
   - **Build Command:** `cargo build --release`
   - **Start Command:** `cargo run --release` or `./target/release/coffe-labs-backend`
   - Never run unoptimized debug builds in production (512 MB RAM ceiling requires `--release` + `mimalloc` + `panic = "abort"` + `strip = true`).
2. **CloudAMQP Little Lemur Connection Budget (Max 20 Connections):**
   - **NEVER** open a new AMQP TCP connection per HTTP request.
   - Maintain a single managed connection (or tiny bounded pool <= 2-3 connections max) initialized at startup and shared via `AppState`, multiplexing lightweight channels (`create_channel`) with clean locking/reconnect recovery.
3. **Durable AMQP Reliability Contract:**
   - Declare exchanges (`Direct` / `Topic`) and queues with `durable: true` and publish JSON payloads with `delivery_mode(2)` (persistent) + `content_type("application/json")`.
   - Enforce `basic_qos(5, ...)` on in-process background consumers (`tokio::spawn`) to cap RAM usage.
   - Always call `basic_ack` after verified processing, or `basic_nack(..., requeue: false)` with structured `tracing::error!` / `warn!` logging on malformed/poison payloads.
   - Never fail server boot when `RABBITMQ_URL` is unset or temporarily unreachable; log a warning and retry with exponential backoff (`tokio::time::sleep`).

---

## 7. Gamification & Anti-Exploit Standards
1. **Idempotent XP Claims:** Every endpoint that grants XP or unlocks achievements MUST require an `X-Idempotency-Key` header or signed session token (`brew_session_id`) validated against `xp_ledger_transactions`.
2. **Server-Authoritative Math:** The Flutter client NEVER calculates or submits XP values. The client only submits verifiable domain events (e.g., `BrewSessionCompleted { session_id, actual_duration_ms }`), and the Rust `GamificationService` computes XP and tier multipliers (`Trial`: 1.0x, `Pro`: 1.5x, `Business`: 2.0x) inside a single SQLx database transaction (`tx.begin()`).
3. **Non-Blocking Audio Mixing (Flutter):** Gamification SFX (`just_audio`) must always use audio ducking (`mixWithOthers`) so celebratory sounds never interrupt active `flutter_tts` brewing instructions.

---

## 8. Structured Observability & Verification Discipline
1. **Structured `tracing`:** Emit contextual fields (`tracing::info!(exchange = %..., routing_key = %..., "...")`) across HTTP requests, DB queries, and broker consumers rather than `println!`.
2. **Verification Before Reporting:** Every Rust change must pass `cargo check` (and unit/integration tests in `tests/` with `cargo test --lib --tests` under offline/sandbox-safe assumptions) and verify the `< 200 LOC` limit and zero production `.unwrap()` rule.
