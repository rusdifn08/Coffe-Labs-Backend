# SKILL.md — Coffee Labs Enterprise Rust & Flutter Architecture Standards

## 1. Core Identity & Philosophy
This repository is a high-performance, ultra-secure, low-memory Rust (Axum) backend and Flutter Mobile Application, powered by **NeonDB Serverless PostgreSQL**.
Every line of code must adhere to three pillars:
1. **Extreme Modularity:** Small, single-responsibility files (Max 200 LOC per `.rs` file).
2. **Defense-in-Depth Security:** Zero-trust input validation, memory-safe secrets, strict RBAC, and anti-abuse rate limiting.
3. **Mechanical Sympathy:** Minimal heap allocations, zero-copy parsing where possible, and serverless-aware database pooling.

---

## 7. Gamification & Anti-Exploit Standards
1. **Idempotent XP Claims:** Every endpoint that grants XP or unlocks achievements MUST require an `X-Idempotency-Key` header or signed session token (`brew_session_id`) validated against `xp_ledger_transactions`.
2. **Server-Authoritative Math:** The Flutter client NEVER calculates or submits XP values. The client only submits verifiable domain events (e.g., `BrewSessionCompleted { session_id, actual_duration_ms }`), and the Rust `GamificationService` computes XP and tier multipliers (`Trial`: 1.0x, `Pro`: 1.5x, `Business`: 2.0x) inside a single SQLx database transaction (`tx.begin()`).
3. **Non-Blocking Audio Mixing (Flutter):** Gamification SFX (`just_audio`) must always use audio ducking (`mixWithOthers`) so celebratory sounds never interrupt active `flutter_tts` brewing instructions.
