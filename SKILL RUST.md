# SKILL.md — Enterprise Rust Axum Backend Design System & Architecture

## 1. Core Identity & Philosophy
This repository is a high-performance, ultra-secure, low-memory Rust (Axum) backend serving a Flutter Mobile Application, powered by **NeonDB Serverless PostgreSQL**.
Every line of code must adhere to three pillars:
1. **Extreme Modularity:** Small, single-responsibility files (Max 200 LOC per file).
2. **Defense-in-Depth Security:** Zero-trust input validation, memory-safe secrets, strict RBAC, and anti-abuse rate limiting.
3. **Mechanical Sympathy:** Minimal heap allocations, zero-copy parsing where possible, and serverless-aware database pooling.

---

## 2. Mandatory Directory Structure
Never dump code into `main.rs` or flat files. Every domain feature must follow this exact modular blueprint:

```text
src/
├── main.rs                      # Entrypoint ONLY: initializes mimalloc, tracing, config, and boots server (< 80 LOC)
├── app.rs                       # Router composition & global Tower middleware stack
├── core/                        # Shared infrastructure & cross-cutting concerns
│   ├── mod.rs
│   ├── config.rs                # Env parsing using `secrecy::SecretString`
│   ├── db.rs                    # NeonDB PgPool setup, health check, & retry logic
│   ├── error.rs                 # Global `AppError` & sanitized HTTP JSON responses
│   ├── extractors.rs            # Custom extractors: `ValidatedJson<T>`, `AuthUser`, `RequireTier`
│   ├── middleware.rs            # Rate limiting (`tower_governor`), CORS, Security Headers, Request ID
│   ├── pagination.rs            # Cursor-based pagination structs for mobile infinite scroll
│   └── security.rs              # Argon2id hashing, Ed25519 JWT signing, Refresh Token family logic
└── modules/                     # Domain Services (Strictly Isolated)
    ├── mod.rs
    ├── auth/                    # Login, Register, OAuth, Token Refresh, Device Sessions
    │   ├── mod.rs
    │   ├── routes.rs            # Axum router definitions for /api/v1/auth
    │   ├── handlers.rs          # HTTP transport layer ONLY (extracts DTO, calls service, returns JSON)
    │   ├── service.rs           # Pure business logic, token rotation, & security checks
    │   ├── repository.rs        # Pure SQLx queries to NeonDB (zero business logic)
    │   ├── dto.rs               # Request/Response structs with `garde`/`validator` rules
    │   ├── models.rs            # Database row structs (`sqlx::FromRow`)
    │   └── error.rs             # Auth-specific domain errors
    ├── rbac/                    # Account Tiers (Trial, Pro, Business), Quotas, Feature Flags
    │   ├── mod.rs
    │   ├── routes.rs
    │   ├── handlers.rs
    │   ├── service.rs           # Subscription state machine, trial expiry, & tier capability checks
    │   ├── repository.rs
    │   ├── dto.rs
    │   ├── models.rs            # Enums: `AccountTier { Trial, Pro, Business }`, `PermissionScope`
    │   └── guard.rs             # Axum middleware/extractor enforcing tier rules per endpoint
    └── social/                  # In-App Social Media (Posts, Recipe Shares, Likes, Comments, Follows)
        ├── mod.rs
        ├── routes.rs
        ├── handlers/            # Split handlers into sub-files when endpoints grow!
        │   ├── mod.rs
        │   ├── feed_handler.rs
        │   ├── post_handler.rs
        │   └── interaction_handler.rs
        ├── services/
        │   ├── mod.rs
        │   ├── feed_service.rs
        │   └── post_service.rs
        ├── repositories/
        │   ├── mod.rs
        │   ├── feed_repo.rs
        │   └── post_repo.rs
        ├── dto.rs
        └── models.rs