pub const NAV_SHELL_HTML: &str = r#"
  <header class="header-card">
    <div class="brand">
      <div class="logo">☕</div>
      <div class="title">
        <h1>Coffee Labs — Command & Telemetry Hub</h1>
        <p id="sub-banner">Rust Axum Enterprise • NeonDB Serverless • Ed25519 JWT • RoaringBitmap • 512MB RAM Guard</p>
      </div>
    </div>
    <div class="actions">
      <span id="conn-badge" class="badge ok">● LIVE STREAM (2s)</span>
      <button class="btn primary" onclick="runApiBenchmark()">⚡ Benchmark Mobile API</button>
      <button class="btn" onclick="refreshAllPanels()">🔄 Refresh</button>
      <button class="btn" onclick="resetTelemetry()">🧹 Reset Metrics</button>
      <a class="btn espresso" href="/api/v1/openapi.json" target="_blank">📄 OpenAPI 3.1 JSON</a>
    </div>
  </header>

  <nav class="nav-pill-bar" aria-label="Dashboard Navigation">
    <div class="nav-group">
      <button class="nav-btn active" data-tab="apm" onclick="switchDashTab('apm', true)">
        ⚡ Monitor API Request
        <span class="nav-count" id="nav-badge-reqs">0</span>
      </button>
      <button class="nav-btn" data-tab="monitoring" onclick="switchDashTab('monitoring', true)">
        🟢 User Online &amp; Aktivitas
        <span class="nav-count live" id="nav-badge-online">0 Online</span>
      </button>
      <button class="nav-btn" data-tab="admin" onclick="switchDashTab('admin', true)">
        👥 Admin Users &amp; Auth
        <span class="nav-count" id="nav-badge-users">Admin</span>
      </button>
      <button class="nav-btn" data-tab="services" onclick="switchDashTab('services', true)">
        🧩 Services &amp; Infra
        <span class="nav-count" id="nav-badge-services">22 Modul</span>
      </button>
    </div>
    <div style="display:flex;gap:8px;align-items:center">
      <span class="badge espresso" id="nav-uptime-pill">⏱️ Uptime: 0s</span>
      <span class="badge info" id="nav-err-pill">Guard: 0 err</span>
    </div>
  </nav>

  <div class="auth-bar" id="mon-auth-bar">
    <div>
      <strong style="color:var(--roast);font-size:13px">🛡️ Sesi Admin &amp; Business Command Center</strong>
      <div style="font-size:11.5px;color:var(--text-muted)" id="mon-auth-status">Ringkasan agregat aktif. Masuk akun Admin untuk membuka Live User Online, 360° Dossier &amp; /api/admin/users.</div>
    </div>
    <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center" id="mon-login-controls">
      <input id="mon-user" class="inp" placeholder="Username (rusdifn08)" style="width:150px">
      <input id="mon-pass" class="inp" type="password" placeholder="Password" style="width:130px" onkeydown="if(event.key==='Enter')monLogin()">
      <button class="btn espresso" onclick="monLogin()">🔓 Masuk Dashboard</button>
    </div>
    <div style="display:none;gap:8px;flex-wrap:wrap;align-items:center" id="mon-unlocked-controls">
      <span class="badge espresso" id="mon-me-pill">@admin</span>
      <span class="ram-pill" id="mon-ram-guard">MPSC 0/512 • RAM Aman</span>
      <button class="btn" onclick="switchDashTab('monitoring', true)">🟢 Buka User Online</button>
      <button class="btn" onclick="monLogout()">Keluar</button>
    </div>
  </div>
"#;
