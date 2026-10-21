# Coffe Labs Backend

Backend API performa tinggi untuk ekosistem **Coffee Labs**, dibangun menggunakan **Rust**, **Axum**, **SQLx**, dan **PostgreSQL (Neon Serverless)**.

---

## 🚀 Fitur Utama

1. **Authentication & Multi-Role Approval System**
   - Registrasi & Login pengguna dengan hashing aman (**Argon2**) dan **JWT**.
   - Sistem persetujuan akun admin oleh super admin (`pending` -> `approved` / `rejected`).
   - Proteksi middleware JWT pada endpoint admin dan fitur privat.

2. **The Hybrid 3-Tier Gatekeeper & Dynamic App Updates**
   - Semantic Versioning Engine: Cek versi aplikasi mobile secara dinamis (`/api/v1/version-check`).
   - Tiga tingkatan pembaruan:
     - **Tier 1 (OTA Instant)**: Shorebird code push untuk bug fix darurat.
     - **Tier 2 (Direct In-App APK)**: Download APK langsung di dalam aplikasi untuk pembaruan minor.
     - **Tier 3 (External Store)**: Arahkan ke Google Play Store / App Store untuk pembaruan mayor arsitektur.
   - Admin CRUD dashboard untuk konfigurasi versi dan toggle mode pemeliharaan (*maintenance mode*).

3. **User Feedback & Bug Reporting System**
   - Pengiriman feedback, rating, dan laporan kendala dari pengguna aplikasi mobile.
   - Manajemen status feedback oleh admin (`open`, `in_progress`, `resolved`, `dismissed`).

4. **Social Coffee Community & Recipe Sharing Platform**
   - Linimasa komunitas dengan snapshot resep seduh lengkap (rasio, suhu, berat kopi & air, tasting notes).
   - Fitur **Copy Recipe** untuk menduplikasi resep komunitas langsung ke resep kustom pengguna.
   - Interaksi sosial: Toggle Like/Unlike dengan counter atomik, dan sistem komentar komunitas.

---

## 🛠️ Tech Stack

* **Bahasa**: [Rust](https://www.rust-lang.org/) (Edition 2021)
* **Web Framework**: [Axum](https://github.com/tokio-rs/axum) (Tokio asynchronous runtime)
* **Database & ORM**: [SQLx](https://github.com/launchbadge/sqlx) dengan PostgreSQL Neon
* **Keamanan**: Argon2 password hashing, JSON Web Tokens (JWT)
* **Serialisasi**: Serde & Serde JSON
* **Logging & Tracing**: `tracing` & `tracing-subscriber`

---

## 📋 Struktur Direktori

```text
coffe-labs-backend/
├── src/
│   ├── main.rs            # Entry point Axum & CORS setup
│   ├── config.rs          # Environment & konfigurasi runtime
│   ├── db.rs              # Koneksi PostgreSQL & auto migration
│   ├── models.rs          # Rust structs & database entity mapping
│   ├── handlers.rs        # Route controller & business logic
│   └── middleware.rs      # JWT Auth & Gatekeeper interceptor
├── Cargo.toml             # Dependencies & metadata paket
├── Cargo.lock
├── .env.example           # Contoh konfigurasi environment
└── README.md
```

---

## ⚡ Menjalankan Backend Secara Lokal

### Prasyarat
* [Rust & Cargo](https://rustup.rs/) (versi 1.75 atau lebih baru)
* Akun PostgreSQL (misal [Neon.tech](https://neon.tech))

### Langkah Menjalankan:
1. **Clone repository**:
   ```bash
   git clone https://github.com/rusdifn08/Coffe-Labs-Backend.git
   cd Coffe-Labs-Backend
   ```

2. **Siapkan file `.env`**:
   Salin `.env.example` ke `.env`:
   ```bash
   cp .env.example .env
   ```
   Sesuaikan `DATABASE_URL` dengan database PostgreSQL Anda.

3. **Jalankan server**:
   ```bash
   cargo run --release
   ```
   Server akan otomatis melakukan migrasi skema tabel dan berjalan pada `http://0.0.0.0:3000`.

---

## 📚 Ringkasan Endpoint API

| Method | Endpoint | Keterangan |
| :--- | :--- | :--- |
| `GET` | `/health` | Healthcheck server status |
| `POST` | `/api/v1/auth/register` | Pendaftaran akun pengguna baru |
| `POST` | `/api/v1/auth/login` | Login & generate JWT token |
| `GET` | `/api/v1/auth/users` | List akun pending (Khusus Admin) |
| `POST` | `/api/v1/auth/users/:id/accept` | Menyetujui pendaftaran admin |
| `POST` | `/api/v1/auth/users/:id/decline`| Menolak pendaftaran admin |
| `GET` | `/api/v1/version-check` | Cek status versi aplikasi mobile |
| `POST` | `/api/v1/version-configs` | Simpan/ubah konfigurasi versi (Admin) |
| `POST` | `/api/v1/feedback` | Kirim feedback dari mobile |
| `GET` | `/api/v1/feedback` | Ambil semua feedback (Admin) |
| `GET` | `/api/v1/social/posts` | Linimasa feed Social Coffee |
| `POST` | `/api/v1/social/posts` | Buat postingan & lampirkan resep |
| `POST` | `/api/v1/social/posts/:id/like` | Toggle like postingan |
| `GET` | `/api/v1/social/posts/:id/comments` | Ambil komentar postingan |
| `POST` | `/api/v1/social/posts/:id/comments` | Kirim komentar pada postingan |
| `POST` | `/api/v1/social/posts/:id/copy-recipe` | Salin resep ke koleksi pribadi |

---

## 📄 Lisensi
Hak Cipta © 2026 Coffee Labs. Semua hak dilindungi undang-undang.
