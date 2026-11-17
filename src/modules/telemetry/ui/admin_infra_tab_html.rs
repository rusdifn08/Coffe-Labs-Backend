pub const ADMIN_TAB_HTML: &str = r#"
<div id="tab-admin" class="tab-pane hidden">
  <section class="grid-4">
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Total Pengguna Terdaftar</div>
          <div class="card-val" id="adm-total-users">0</div>
        </div>
        <div class="kpi-icon espresso">👥</div>
      </div>
      <div class="card-sub"><span>Endpoint: <b>/api/admin/users/stats</b></span></div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Registrasi Baru (7 Hari)</div>
          <div class="card-val" id="adm-new-7d">0</div>
        </div>
        <div class="kpi-icon amber">🔔</div>
      </div>
      <div class="card-sub"><span>Akun baru minggu ini</span></div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Barista Aktif (Approved)</div>
          <div class="card-val" id="adm-active-users">0</div>
        </div>
        <div class="kpi-icon green">✅</div>
      </div>
      <div class="card-sub"><span>Terverifikasi aktif</span></div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Pending / Declined</div>
          <div class="card-val" id="adm-pending-users">0</div>
        </div>
        <div class="kpi-icon hot">⏳</div>
      </div>
      <div class="card-sub"><span>Menunggu persetujuan Admin</span></div>
    </div>
  </section>

  <section class="card" style="margin-bottom:18px" id="adm-me-card">
    <div class="section-head">
      <h2>🔐 Sesi Akun Saat Ini (/api/v1/auth/me)</h2>
      <span class="badge info" id="adm-me-badge">Belum Login</span>
    </div>
    <div id="adm-me-body" style="font-size:12.5px;color:var(--text-muted)">
      Masuk sebagai Admin pada bar di atas untuk memverifikasi profil JWT Ed25519 dan mengelola user database.
    </div>
  </section>

  <section class="card">
    <div class="section-head">
      <h2>🛡️ Manajemen Pengguna &amp; Tier RBAC (/api/admin/users)</h2>
      <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center">
        <input id="adm-search" class="inp" placeholder="Cari username / nama..." oninput="renderAdminUsersTable()" style="width:190px">
        <div class="filter-chips">
          <button class="chip-btn active" onclick="setAdminStatusFilter('ALL', this)">Semua</button>
          <button class="chip-btn" onclick="setAdminStatusFilter('pending', this)">Pending</button>
          <button class="chip-btn" onclick="setAdminStatusFilter('approved', this)">Approved</button>
          <button class="chip-btn" onclick="setAdminStatusFilter('declined', this)">Declined</button>
        </div>
        <button class="btn" onclick="loadAdminUsersPanel()">🔄 Muat Ulang</button>
      </div>
    </div>
    <div style="overflow-x:auto">
      <table>
        <thead>
          <tr>
            <th>Barista</th><th>Role &amp; Status</th><th>Account Tier</th>
            <th>Dibuat</th><th>Aksi Cepat</th>
          </tr>
        </thead>
        <tbody id="adm-users-tbody">
          <tr><td colspan="5" style="text-align:center;color:var(--text-muted);padding:20px">Masuk Admin di atas untuk memuat daftar pengguna dari /api/admin/users.</td></tr>
        </tbody>
      </table>
    </div>
  </section>
</div>
"#;

pub const SERVICES_TAB_HTML: &str = r#"
<div id="tab-services" class="tab-pane hidden">
  <section class="grid-4">
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">NeonDB PostgreSQL 16</div>
          <div class="card-val" id="inf-db-status">Checking...</div>
        </div>
        <div class="kpi-icon water">🗄️</div>
      </div>
      <div class="card-sub"><span id="inf-db-meta">Rustls Pool</span></div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Redis Cache &amp; Streams</div>
          <div class="card-val" id="inf-redis-status">Checking...</div>
        </div>
        <div class="kpi-icon hot">⚡</div>
      </div>
      <div class="card-sub"><span id="inf-redis-meta">/api/v1/health/redis</span></div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">RabbitMQ Event Bus</div>
          <div class="card-val" id="inf-mq-status">Checking...</div>
        </div>
        <div class="kpi-icon amber">📨</div>
      </div>
      <div class="card-sub"><span id="inf-mq-meta">/api/v1/health/rabbitmq</span></div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">WebSocket &amp; Moka L1 Cache</div>
          <div class="card-val" id="inf-cache-entries">0 Entries</div>
        </div>
        <div class="kpi-icon green">🧩</div>
      </div>
      <div class="card-sub"><span id="inf-fcm-cloud">FCM &amp; Cloudinary status</span></div>
    </div>
  </section>

  <section class="card">
    <div class="section-head">
      <h2>🧩 Modular Backend Services &amp; Mobile Feature Architecture</h2>
      <div style="display:flex;gap:8px;align-items:center">
        <input id="svc-search" class="inp" placeholder="Filter modul/route..." oninput="applyServicesFilter()" style="width:190px">
        <span class="badge ok">All Services Operational</span>
      </div>
    </div>
    <div style="overflow-x:auto">
      <table>
        <thead>
          <tr>
            <th>Domain Module</th><th>Service Name</th><th>Base Route</th>
            <th>Mobile Feature Integration</th><th>Security &amp; Guard Layer</th><th>State</th>
          </tr>
        </thead>
        <tbody id="services-tbody"></tbody>
      </table>
    </div>
  </section>
</div>
"#;
