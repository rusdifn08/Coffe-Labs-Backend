# ☕ Coffee Labs — Enterprise Rust Axum Backend v2.0.0

<div align="center">

![Coffee Labs Backend Banner](https://raw.githubusercontent.com/rusdifn08/coffe-labs-mobile/main/assets/images/logo.png)

**High-Performance, Low-Latency Rust (Axum 0.8) Microservices Engine for Specialty Coffee Ecosystems**  
*Powering Mobile Applications, Cognitive Sommelier AI, IoT Slow Bar Hardware, & Real-Time APM Observability.*

[![Rust](https://img.shields.io/badge/Rust-1.80+-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Axum](https://img.shields.io/badge/Framework-Axum%200.8-blue)](https://github.com/tokio-rs/axum)
[![Database](https://img.shields.io/badge/Database-Serverless%20NeonDB%20Postgres-00E699?logo=postgresql&logoColor=white)](https://neon.tech)
[![Security](https://img.shields.io/badge/Auth-Ed25519%20%2B%20Argon2id-green)](https://en.wikipedia.org/wiki/EdDSA)
[![Allocator](https://img.shields.io/badge/Allocator-mimalloc-blueviolet)](https://github.com/microsoft/mimalloc)
[![AI/OCR](https://img.shields.io/badge/Cognitive%20AI-Sommelier%20v2.0-gold)](https://github.com/rusdifn08/coffe-labs-backend)
[![OpenAPI](https://img.shields.io/badge/OpenAPI-3.1%20Utoipa-85EA2D?logo=openapi-initiative&logoColor=black)](http://localhost:3000/api/v1/openapi.json)
[![License](https://img.shields.io/badge/License-Proprietary-red)](#license)

[Arsitektur Inti](#-arsitektur-inti--engineering-pillars) •
[Cognitive Sommelier AI v2.0](#-cognitive-sommelier-ai-v20--ocr-engine) •
[Domain Modules & Endpoint Directory](#-domain-modules--endpoint-directory) •
[Observability & APM Dashboard](#-real-time-observability--apm-dashboard) •
[Database & NeonDB Serverless Pool](#-database--skema-neondb-serverless) •
[Setup & Instalasi](#-setup--instalasi-lokal) •
[Pengujian & Verifikasi](#-pengujian--quality-assurance)

</div>

---

## 🏛️ Arsitektur Inti & Engineering Pillars

Backend **Coffee Labs v2.0.0** dirancang dengan standar *Extreme Systems Programming* untuk mencapai latensi sub-milidetik, efisiensi memori tingkat tinggi (*zero unnecessary heap allocation*), serta ketahanan terhadap kegagalan jaringan pada arsitektur *serverless cloud*.

```
                      ┌─────────────────────────────────────────┐
                      │    Coffee Labs Mobile / POS / Web       │
                      └────────────────────┬────────────────────┘
                                           │ HTTP/1.1 & HTTP/2 (TLS)
                                           ▼
                      ┌─────────────────────────────────────────┐
                      │     Tower Middleware Stack & Security    │
                      │ ─────────────────────────────────────── │
                      │ • tower_governor (Sliding IP Rate Limit)│
                      │ • Request ID & Security Headers         │
                      │ • Brotli (br) & Gzip Response Encoder   │
                      │ • Request Body Limit (4MB Guard)        │
                      │ • Microsecond Telemetry & APM Timer     │
                      └────────────────────┬────────────────────┘
                                           │
                                           ▼
                      ┌─────────────────────────────────────────┐
                      │       Axum 0.8 Router Composition       │
                      │  (Zero-Cost Pattern & ValidatedJson<T>) │
                      └───────┬─────────────────────────┬───────┘
                              │                         │
            ┌─────────────────┴────────┐       ┌────────┴────────────────┐
            ▼                          ▼       ▼                         ▼
   ┌─────────────────┐        ┌─────────────┐ ┌───────────────┐ ┌───────────────┐
   │ Auth & Security │        │  Cognitive  │ │  Social, POS  │ │ APM Dashboard │
   │ Ed25519 + JWT   │        │  Sommelier  │ │  & Recipes    │ │  & Telemetry  │
   │ Argon2id Crypto │        │  AI Engine  │ │  Domain Logic │ │ DashMap P99   │
   └────────┬────────┘        └──────┬──────┘ └───────┬───────┘ └───────┬───────┘
            │                        │                │                 │
            └────────────────────────┼────────────────┴─────────────────┘
                                     │ SQLx 0.8 Connection Pool
                                     ▼
                      ┌─────────────────────────────────────────┐
                      │     NeonDB Serverless PostgreSQL        │
                      │  (Scale-to-Zero, Cold-Start Resilient)  │
                      └─────────────────────────────────────────┘
```

### 1. Mechanical Sympathy & Optimasi Memori Ekstrem
* **`mimalloc` (`#[global_allocator]`)**: Pengganti alokator sistem bawaan OS yang dirancang oleh Microsoft untuk mengurangi fragmentasi memori *heap* dan menghasilkan konkurensi alokasi multi-core berkecepatan tinggi.
* **Zero-Copy & Small-String Optimization (`compact_str`)**: String pendek disimpan langsung secara *inline* di *stack* tanpa alokasi memori dinamis di *heap*.
* **Lock-Free Telemetry Registry (`DashMap`)**: Pencatatan metrik performa latensi endpoint API dengan *sharded concurrent hashmap* tanpa *mutex lock contention*.
* **Streaming Compression (`CompressionLayer`)**: Kompresi otomatis berbasis **Brotli (`br`)** dan **Gzip** untuk meminimalkan *bandwidth* data seluler aplikasi *mobile*.

### 2. Defense-in-Depth Security (Zero-Trust)
* **Kriptografi Asimetris Ed25519 (`EdDSA`)**: Token otentikasi JWT ditandatangani menggunakan *keypair* Ed25519 berstandar militer. Verifikasi token berjalan secepat kilat tanpa perlu membebani database utama.
* **Argon2id (`v0x13`) Password Hashing**: Hashing kata sandi tahan serangan GPU, ASIC, dan *side-channel attacks*. Dilengkapi sistem migrasi transparan otomatis dari hash warisan.
* **Refresh Token Family & Reuse Attack Detection**: Refresh token disimpan dalam format hash SHA-256 di database PostgreSQL. Jika terjadi percobaan penggunaan ulang (*token replay/theft*), seluruh *family token* perangkat terkait langsung dicabut seketika (*revoked*).
* **Zero-Trust Payload Extraction (`ValidatedJson<T>`)**: Extractor khusus Axum yang memvalidasi setiap payload JSON sebelum mengeksekusi *handler* bisnis melalui aturan deklaratif `validator`.
* **Proteksi DDoS & Rate Limiting (`tower_governor`)**: Pembatasan kuota request per detik berbasis IP dengan algoritma *sliding window* yang mengembalikan respons `429 Too Many Requests`.
* **Memory-Safe Secrets (`secrecy::SecretString`)**: Pencegahan kebocoran kredensial database, JWT seed, dan private key di dalam *log tracing* atau *memory dump*.

### 3. Arsitektur Bersih Berbasis Domain (< 200 LOC per File)
Setiap modul fungsional diisolasi secara ketat dan mematuhi prinsip tanggung jawab tunggal (*Single Responsibility Principle*):
* `routes.rs`: Definisi URI endpoint dan *middleware binding*.
* `handlers.rs`: Ekstraksi request HTTP dan pemetaan respons JSON.
* `service.rs`: Logika komputasi bisnis, validasi aturan domain, dan *caching*.
* `repository.rs`: Kueri SQL berkinerja tinggi menggunakan `sqlx` dengan parameter terikat.
* `dto.rs`: Struktur data transfer request/response dengan derivasi `Validate`, `Serialize`, `Deserialize`, dan `ToSchema`.
* `models.rs`: Representasi entitas tabel database.
* `error.rs`: Pemetaan error domain spesifik ke dalam `AppError` tanpa mengekspos rincian kueri SQL internal ke publik.

---

## 🧠 Cognitive Sommelier AI v2.0 & OCR Engine

Modul `bean_scanner` (`/api/v1/beans/scan-resolve`) merupakan engine kecerdasan buatan visi dan semantik yang memproses teks hasil OCR kemasan kopi dan mengonversinya menjadi profil seduh terstruktur secara deterministik dan presisi.

```
       [Raw OCR Text + Bounding Blocks from Mobile ML Kit]
                                │
                                ▼
         ┌──────────────────────────────────────────────┐
         │       1. Heuristic Noise Filter Engine       │
         │   • Menghapus stop words & jargon pemasaran  │
         │   • Membersihkan tanggal sangrai & berat     │
         │   • Mengabaikan barcode, izin edar, & ISO    │
         └──────────────────────┬───────────────────────┘
                                │
                                ▼
         ┌──────────────────────────────────────────────┐
         │     2. Spatial OCR & Typographic Weigher     │
         │   • Font-Height Scoring: Judul = teks besar  │
         │   • Spatial Position: Roastery = header atas │
         │   • Body Layout: Tasting notes = teks kecil  │
         └──────────────────────┬───────────────────────┘
                                │
                                ▼
         ┌──────────────────────────────────────────────┐
         │    3. Knowledge Graph & Terroir Dictionary   │
         │   • Indonesian Terroirs (Malabar, Halu, dsb) │
         │   • International Origins (Yirgacheffe, dsb) │
         │   • Roastery Taxonomy (BAK, Otten, Space)    │
         │   • Variety Catalog (Typica, Geisha, Kartika)│
         └──────────────────────┬───────────────────────┘
                                │
                                ▼
         ┌──────────────────────────────────────────────┐
         │    4. Jaro-Winkler Fuzzy Flavor Matcher      │
         │   • SCA 92-Flavor Wheel Matcher (Score >=0.83│
         │   • Toleransi saltik & kesalahan OCR kamera  │
         └──────────────────────┬───────────────────────┘
                                │
                                ▼
         ┌──────────────────────────────────────────────┐
         │    5. Cross-Inference & Recipe Engine        │
         │   • Proses Wine => Inferred Notes (Winey,Rum)│
         │   • Pasangan Resep Juara & Suhu Air Seduh    │
         │   • Penugasan Bounding Box Role & Warna Hex  │
         └──────────────────────┬───────────────────────┘
                                │
                                ▼
      [Resolved Coffee Bean Profile + Interactive Colored Overlays]
```

### Komponen Teknis Cognitive Sommelier:
1. **Noise Filter (`noise_filter.rs`)**:
   - Membersihkan teks non-esensial (*"FRESH BEAN", "BETTER BREW", "ROASTING DATE", "BEST BEFORE", "NETTO 200G", "100% ARABICA", "P-IRT"*) menggunakan pencocokan substring cepat dan regex.
2. **Knowledge Graph (`knowledge_graph.rs`)**:
   - Kamus database terroirs Indonesia (Malabar, Gunung Halu, Wamena Bpiri, Ijen, Gayo, Kintamani, Toraja, Flores Bajawa, dll.) dan global (Yirgacheffe, Sidamo, Huila, Nariño, Gesha Village).
   - Katalog roastery terdaftar (BAK Coffee Bean, Otten Coffee, Space Roastery, Dua Kopi, Fugol Coffee, Tanamera, dll.).
   - Database varietas kopi (*Typica, Bourbon, Geisha, SL28, Sigarar Utang, Kartika, Caturra, Pacamara*).
3. **Jaro-Winkler Fuzzy Flavor Matcher (`fuzzy_flavor_matcher.rs`)**:
   - Membandingkan kata hasil OCR dengan 92 deskriptor rasa resmi SCA (*Specialty Coffee Association*) menggunakan algoritma jarak string Jaro-Winkler (`strsim::jaro_winkler`).
   - Kata dengan nilai kecocokan $\ge 0.83$ secara otomatis dipetakan ke *tasting notes* baku (contoh: *"stobery"* $\rightarrow$ *"Strawberry"*, *"blubery"* $\rightarrow$ *"Blueberry"*).
4. **Cognitive Cross-Inference Engine (`cognitive_resolver.rs`)**:
   - **Inference Proses Pasca-Panen**: Jika mendeteksi kata *"Wine"* atau *"Anaerobic"*, engine secara otomatis menyimpulkan karakteristik rasa fermentasi khas (*Winey, Fermented Grapes, Dark Cherry, Rum*) bila tidak tercantum di teks label.
   - **Estimasi Ketinggian (Altitude Inference)**: Menghitung rata-rata elevasi tanaman berdasarkan origin (contoh: *Wamena Bpiri* $\rightarrow$ `1,600 - 2,000 masl`).
5. **Spatial Bounding Box Classifier (`dto.rs`)**:
   - Mengelompokkan setiap koordinat teks hasil OCR ke dalam peran fungsional dan memberikan warna hex visual:
     - 🟡 **Roastery Brand (`#D4AF37`)**: Brand / Nama Roastery
     - 🟢 **Origin / Terroir (`#4E8752`)**: Daerah asal kopi & Judul
     - 🔴 **Process (`#9E2A3B`)**: Proses pasca-panen (Washed, Natural, Wine, Honey)
     - 🔵 **Tasting Notes (`#3B82A6`)**: Karakteristik rasa seduhan
     - 🟣 **Altitude (`#5C6BC0`)**: Ketinggian tanam (*masl*)
     - 🌲 **Variety (`#2E7D32`)**: Varietas tanaman kopi
6. **Title Composer (`title_composer.rs`)**:
   - Menyusun judul kopi yang rapi dan konsisten sesuai kaidah industri (*"Roastery - Origin Process"*), menghilangkan duplikasi kata yang redundan.
7. **Recipe Recommendation Matcher**:
   - Menghubungkan profil biji hasil scan dengan metode seduh juara dunia yang paling optimal (contoh: Proses *Wine* otomatis disarankan metode *Tetsu Kasuya 4:6* pada suhu 91°C untuk menonjolkan keasaman anggur yang kompleks dan rasa manis buah matang).

---

## 📂 Struktur Direktori Proyek

```text
coffe-labs-backend/
├── Cargo.toml                   # Dependensi Rust, compiler optimization & profile
├── tests/
│   └── cognitive_bean_scanner_test.rs # 4/4 Integration Tests Cognitive AI Scanner
├── src/
│   ├── lib.rs                   # Library crate root (app, core, modules exposure)
│   ├── main.rs                  # Binary entrypoint: mimalloc, tracing & server init
│   ├── app.rs                   # Router composition & global Tower middleware stack
│   ├── core/                    # Shared Infrastructure & Cross-Cutting Concerns
│   │   ├── config.rs            # Env config parser dengan secrecy::SecretString
│   │   ├── db.rs                # Serverless PgPool configuration (NeonDB tuned)
│   │   ├── migrations.rs        # Idempotent DB schema & index creation
│   │   ├── seeds.rs             # Default admin, seed users, beans & recipes
│   │   ├── error.rs             # AppError & sanitized JSON HTTP responses
│   │   ├── extractors.rs        # ValidatedJson<T>, AuthUser, AdminUser, TierGuard
│   │   ├── middleware.rs        # tower_governor, Security Headers, APM timer
│   │   ├── pagination.rs        # Base64 cursor-based pagination
│   │   ├── security.rs          # Argon2id & Ed25519 JWT keypair manager
│   │   └── telemetry.rs         # Lock-free DashMap percentile latency registry
│   └── modules/                 # 13 Isolated Domain Microservices
│       ├── auth/                # Login, Register, Refresh Token Rotation
│       ├── rbac/                # Account Tiers, Quotas, Billing & Gatekeeper
│       ├── social/              # Community Feed, Likes, Comments, Follows
│       ├── users/               # Barista Profile, Gear Sync & Admin Approval
│       ├── recipes/             # Champion Brew Recipes & Cloud Sync
│       ├── brew_logs/           # Brew History, Sensory Cupping & Extraction TDS
│       ├── beans/               # Curated Beans Catalog & Roastery Radar
│       ├── bean_scanner/        # Cognitive Sommelier AI & OCR Engine
│       │   ├── dto.rs           # BoundingBox, Role badges & Request/Response DTO
│       │   ├── handlers.rs      # POST /api/v1/beans/scan-resolve handler
│       │   ├── routes.rs        # Scanner router registration
│       │   └── services/        # Sub-engines:
│       │       ├── cognitive_resolver.rs   # Core cognitive reasoning logic
│       │       ├── fuzzy_flavor_matcher.rs # Jaro-Winkler SCA 92-wheel matcher
│       │       ├── knowledge_graph.rs      # Roasteries, Origins & Varietals
│       │       ├── noise_filter.rs         # OCR noise & stopword purifier
│       │       ├── resolver_service.rs     # Database lookup & response composer
│       │       └── title_composer.rs       # Standardized bean title formatter
│       ├── marketplace/         # Specialty Beans Store & Checkout
│       ├── gamification/        # Barista XP Ledger, Streaks & Badges
│       ├── business_pos/        # POS Orders, Menu & Thermal ESC/POS Receipts
│       ├── platform/            # 3-Tier Version Gatekeeper & Shorebird OTA Sync
│       └── telemetry/           # Real-Time APM Dashboard UI & OpenAPI Specification
```

---

## 🌐 Domain Modules & Endpoint Directory

Semua respons API menggunakan amplop JSON standar yang konsisten:
```json
{
  "success": true,
  "message": "Pesan deskriptif keberhasilan operasi",
  "data": { ... }
}
```

### 1. Autentikasi & Keamanan (`/api/v1/auth`)
| Metode | Endpoint | Akses | Deskripsi |
| :--- | :--- | :--- | :--- |
| `POST` | `/api/v1/auth/register` | Publik | Registrasi akun barista baru dengan validasi Argon2id |
| `POST` | `/api/v1/auth/login` | Publik | Login, penerbitan Ed25519 Access Token & Refresh Token |
| `POST` | `/api/v1/auth/refresh` | Publik | Rotasi Refresh Token dengan deteksi *token replay/theft* |
| `POST` | `/api/v1/auth/logout` | Terotentikasi | Pencabutan sesi token aktif perangkat |
| `GET`  | `/api/v1/auth/me` | Terotentikasi | Informasi akun barista yang sedang aktif |

### 2. Cognitive Sommelier AI & Scanner (`/api/v1/beans`)
| Metode | Endpoint | Akses | Deskripsi |
| :--- | :--- | :--- | :--- |
| `POST` | `/api/v1/beans/scan-resolve` | Publik / Auth | **Engine Cognitive Sommelier**: Analisis teks OCR, eliminasi noise, ekstraksi origin, proses, varietas, altitude, fuzzy flavor notes, rekomendasi resep juara, dan pembuatan *classified bounding box*. |

### 3. Katalog Biji Kopi & Cellar (`/api/v1/beans`)
| Metode | Endpoint | Akses | Deskripsi |
| :--- | :--- | :--- | :--- |
| `GET`  | `/api/v1/beans` | Publik | Daftar kurasi biji kopi spesialti (mendukung pencarian & filter) |
| `POST` | `/api/v1/beans` | Terotentikasi | Menambahkan kantong biji kopi baru ke rak digital (*Cellar*) |
| `GET`  | `/api/v1/beans/:id` | Publik | Detail lengkap spesifikasi biji kopi & profil sangrai |
| `PUT`  | `/api/v1/beans/:id` | Terotentikasi | Pembaruan data rasa atau catatan kalibrasi biji kopi |

### 4. Resep Seduh Juara Dunia & Kustom (`/api/v1/recipes`)
| Metode | Endpoint | Akses | Deskripsi |
| :--- | :--- | :--- | :--- |
| `GET`  | `/api/v1/recipes` | Publik | Daftar 200+ resep juara dunia dan resep terverifikasi |
| `POST` | `/api/v1/recipes` | Terotentikasi | Membuat dan mengunggah resep seduh kustom |
| `GET`  | `/api/v1/recipes/:slug`| Publik | Detail resep, tahap interval pour, suhu air, dan grind size |
| `POST` | `/api/v1/recipes/:id/favorite` | Terotentikasi | Menandai resep sebagai favorit |

### 5. Log Seduhan & Jurnal Sensori (`/api/v1/brew-logs`)
| Metode | Endpoint | Akses | Deskripsi |
| :--- | :--- | :--- | :--- |
| `POST` | `/api/v1/brew-logs` | Terotentikasi | Menyimpan sesi seduh baru (TDS, ekstraksi yield, suhu, rasio) |
| `GET`  | `/api/v1/brew-logs` | Terotentikasi | Riwayat log seduhan pengguna dengan pagination |
| `GET`  | `/api/v1/brew-logs/stats` | Terotentikasi | Statistik agregat seduhan mingguan, alat terfavorit & TDS rata-rata |

### 6. Komunitas Sosial & Basecamp (`/api/v1/social`)
| Metode | Endpoint | Akses | Deskripsi |
| :--- | :--- | :--- | :--- |
| `GET`  | `/api/v1/social/feed` | Publik | Feed publik berbasis *cursor pagination* berkinerja tinggi |
| `POST` | `/api/v1/social/posts` | Terotentikasi | Publikasi sesi seduh ke feed komunitas |
| `POST` | `/api/v1/social/posts/:id/like` | Terotentikasi | Like / Unlike kiriman seduhan |
| `POST` | `/api/v1/social/posts/:id/comments` | Terotentikasi | Komentar bersarang (*nested comments*) pada kiriman |

### 7. Slow Bar POS & Bluetooth Struk (`/api/v1/pos`)
| Metode | Endpoint | Akses | Deskripsi |
| :--- | :--- | :--- | :--- |
| `GET`  | `/api/v1/pos/menu` | Terotentikasi | Daftar menu seduhan slow bar dan ketersediaan biji kopi |
| `POST` | `/api/v1/pos/orders` | Terotentikasi | Pembuatan pesanan meja baru dengan parameter seduh khusus |
| `POST` | `/api/v1/pos/orders/:id/receipt-payload` | Terotentikasi | Mengambil payload byte ESC/POS untuk cetak printer Bluetooth |

### 8. Gamifikasi & Level Barista (`/api/v1/gamification`)
| Metode | Endpoint | Akses | Deskripsi |
| :--- | :--- | :--- | :--- |
| `GET`  | `/api/v1/gamification/profile` | Terotentikasi | Level XP barista, peringkat rank, dan streak seduh harian |
| `GET`  | `/api/v1/gamification/badges` | Terotentikasi | Daftar lencana pencapaian seduhan (*Achievements*) |
| `GET`  | `/api/v1/gamification/leaderboard` | Publik | Papan peringkat barista teraktif mingguan |

### 9. Gatekeeper Platform & Shorebird OTA (`/api/v1/platform`)
| Metode | Endpoint | Akses | Deskripsi |
| :--- | :--- | :--- | :--- |
| `GET`  | `/api/v1/platform/version-check` | Publik | Pengecekan kompatibilitas minimum versi mobile & patch OTA |
| `POST` | `/api/v1/platform/feedback` | Terotentikasi | Pengiriman masukan atau laporan bug dari aplikasi |

### 10. Observabilitas & Telemetri APM (`/dashboard` & `/api/v1/telemetry`)
| Metode | Endpoint | Akses | Deskripsi |
| :--- | :--- | :--- | :--- |
| `GET`  | `/dashboard` atau `/` | Publik | **Web UI APM Live**: Dashboard observabilitas real-time |
| `GET`  | `/api/v1/telemetry/metrics` | Publik | Ringkasan metrik latensi P50, P95, P99 & status database |
| `POST` | `/api/v1/telemetry/benchmark` | Publik | Eksekusi benchmark internal konkurensi multi-endpoint |
| `GET`  | `/api/v1/openapi.json` | Publik | Dokumentasi OpenAPI 3.1 Specification JSON |

---

## 📊 Real-Time Observability & APM Dashboard

Backend dilengkapi *Application Performance Monitoring (APM)* bawaan yang dapat diakses langsung melalui peramban web di:
```text
http://localhost:3000/dashboard
```

### Kemampuan APM Dashboard:
* **Live Percentile Latency**: Pelacakan waktu respons per endpoint (`Min`, `Avg`, `P50`, `P95`, `P99`, `Max`) dalam satuan milidetik dengan presisi mikrodetik.
* **Serverless Pool Health**: Status koneksi aktif vs idle pada NeonDB pool, waktu respon ping database, dan jumlah baris tabel domain.
* **Security & Traffic Guard**: Penghitung distribusi status HTTP (`2xx`, `3xx`, `4xx`, `5xx`), request yang ditolak oleh rate-limiter, dan streaming log jejak (*request trace stream*).
* **1-Click Live API Benchmark**: Pengujian konkurensi langsung dari UI dashboard yang menembakkan request paralel ke berbagai endpoint untuk menguji latensi dan throughput server.

---

## 🗄️ Database & Skema NeonDB Serverless

Backend dioptimalkan khusus untuk karakteristik komputasi **Serverless PostgreSQL (NeonDB)** yang memiliki fitur *auto-suspend* / *scale-to-zero*.

### Konfigurasi Pool SQLx (`core/db.rs`):
* `min_connections = 0`: Mengizinkan pool untuk mengosongkan koneksi sepenuhnya saat tidak ada lalu lintas data, sehingga instance serverless dapat masuk ke mode *idle/sleep* untuk menghemat biaya komputasi.
* `max_connections = 15`: Batas maksimum koneksi pool terukur agar tidak melampaui batas kuota koneksi serverless.
* `acquire_timeout = 12s`: Waktu tunggu yang cukup panjang untuk mentolerir fase pemanasan (*cold-start wake-up*) NeonDB tanpa menimbulkan error timeout di sisi aplikasi mobile.
* `idle_timeout = 300s`: Pembersihan otomatis koneksi yang tidak aktif selama 5 menit.
* `max_lifetime = 1800s`: Pencegahan *stale connection* akibat perubahan routing internal cloud provider.
* `test_before_acquire(true)`: Verifikasi keaktifan soket TCP sebelum koneksi dialokasikan ke handler request.

### Indeks Komposit & Integritas Data:
* Indeks komposit berbasis `(user_id, created_at DESC)` untuk query feed dan log seduhan berkecepatan tinggi.
* Indeks teks unik pada slug resep dan kode unik roastery.
* Enkripsi hash SHA-256 pada tabel `refresh_tokens` dengan penandaan status *revocation*.

---

## ⚙️ Setup & Instalasi Lokal

### Prasyarat
* **Rust Toolchain**: Versi `1.80` atau yang lebih baru (`rustup update stable`)
* **PostgreSQL / NeonDB**: URL koneksi PostgreSQL yang valid
* **C++ Build Tools**: Dibutuhkan untuk kompilasi modul alokator memori `mimalloc` di Windows/Linux/macOS

### 1. Klon Repositori
```bash
git clone https://github.com/rusdifn08/coffe-labs-mobile.git
cd coffe-labs-backend
```

### 2. Konfigurasi Environment (`.env`)
Salin file konfigurasi lingkungan atau buat file `.env` di root direktori backend:
```env
# Server Binding
HOST=0.0.0.0
PORT=3000

# Database NeonDB Serverless URL
DATABASE_URL=postgres://username:password@ep-sample-pooler.us-east-2.aws.neon.tech/neondb?sslmode=require

# Security Keys
JWT_SECRET=your-super-secret-ed25519-seed-key-minimum-32-characters-long
ADMIN_SECRET_KEY=admin-bootstrap-secret-change-me-in-production

# Rate Limiting
RATE_LIMIT_PER_SECOND=20
RATE_LIMIT_BURST=40

# Logging
RUST_LOG=info,coffe_labs_backend=debug
```

### 3. Kompilasi & Jalankan (Mode Pengembangan)
```bash
cargo run
```
Server akan menginisialisasi skema tabel database dan data awal (*seeds*) secara otomatis, kemudian mulai mendengarkan koneksi di `http://0.0.0.0:3000`.

### 4. Kompilasi Produksi (High Performance Release)
```bash
cargo build --release
```
Biner teroptimasi akan dihasilkan di `target/release/coffe-labs-backend.exe` (atau biner ELF di Linux) dengan *Link-Time Optimization (LTO)* dan stripping simbol debug untuk ukuran file yang sangat ringkas dan kecepatan eksekusi maksimal.

---

## 🧪 Pengujian & Quality Assurance

Sistem dilengkapi rangkaian pengujian unit dan integrasi untuk memverifikasi performa engine AI, filter noise OCR, kalkulasi sensori rasa, dan keamanan otentikasi.

### 1. Menjalankan Tes Integrasi Cognitive Sommelier AI
```bash
cargo test --test cognitive_bean_scanner_test
```
*Menguji skenario nyata deteksi kemasan: BAK Coffee Bean Malabar Wine, Otten Coffee Wamena Bpiri Washed, eliminasi noise stempel sangrai, dan toleransi saltik Jaro-Winkler.*

### 2. Menjalankan Seluruh Rangkaian Pengujian
```bash
cargo test
```

### 3. Pemeriksaan Kualitas Kode (*Linting*)
```bash
cargo clippy --all-targets --all-features
```

---

## 📄 Lisensi & Hak Cipta

Hak Cipta © 2026 **Coffee Labs**. Seluruh hak cipta dilindungi undang-undang (*All Rights Reserved*).  
Arsitektur sistem didesain dan dipelihara secara profesional untuk standar performa enterprise.

Pemelihara Utama: **[@rusdifn08](https://github.com/rusdifn08)**  
Email Kontak: `rusdifn08@gmail.com`
