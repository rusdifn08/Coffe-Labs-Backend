pub const MONITORING_TAB_HTML: &str = r#"
<div id="tab-monitoring" class="tab-pane hidden">
  <section class="grid-4">
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Pengguna Online Live</div>
          <div class="card-val" id="mon-online-now">0</div>
        </div>
        <div class="kpi-icon green">🟢</div>
      </div>
      <div class="card-sub">
        <span>Total Akun: <b id="mon-total-accs">0</b></span>
        <span id="mon-tier-split">T:0 P:0 B:0</span>
      </div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">DAU / WAU / MAU (RoaringBitmap)</div>
          <div class="card-val" id="mon-dau-wau-mau">0 / 0 / 0</div>
        </div>
        <div class="kpi-icon espresso">📊</div>
      </div>
      <div class="card-sub">
        <span>Stickiness: <b id="mon-stickiness">0%</b></span>
        <span>Bitmaps O(1)</span>
      </div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Rata-rata Screen Time Hari Ini</div>
          <div class="card-val" id="mon-avg-screen">0m</div>
        </div>
        <div class="kpi-icon amber">⏱️</div>
      </div>
      <div class="card-sub">
        <span>Seduh 7H: <b id="mon-brews-7d">0</b></span>
        <span>Idle Guard 180s</span>
      </div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Risiko Churn &amp; Dormant</div>
          <div class="card-val" id="mon-churn-count">0</div>
        </div>
        <div class="kpi-icon hot">🛡️</div>
      </div>
      <div class="card-sub">
        <span>High Risk: <b id="mon-risk-high">0</b></span>
        <span>Dormant: <b id="mon-dormant">0</b></span>
      </div>
    </div>
  </section>

  <section class="card" style="margin-bottom:18px">
    <div class="section-head">
      <h2>📡 Pengguna Online Saat Ini (Live Presence Tracker)</h2>
      <div style="display:flex;gap:8px;align-items:center">
        <span class="badge ok" id="mon-spotlight-badge">● Menunggu Sesi Admin</span>
        <button class="btn" onclick="switchDashTab('admin', true)">👥 Kelola Users (/api/admin/users)</button>
      </div>
    </div>
    <div class="online-strip" id="mon-online-spotlight">
      <div style="font-size:12px;color:var(--text-muted);padding:6px 4px">Masuk akun Admin di atas untuk melihat daftar pengguna yang sedang online di aplikasi mobile.</div>
    </div>
  </section>

  <section class="split">
    <div class="card">
      <div class="section-head">
        <h2>👥 Account &amp; Feature Monitoring (Real-Time)</h2>
        <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center">
          <input id="mon-q" class="inp" placeholder="Cari @username atau nama..." oninput="debouncedLoadAccounts()" style="width:170px">
          <select id="mon-tier" class="inp" onchange="loadAccounts(1)">
            <option value="">Semua Tier</option>
            <option value="trial">Trial</option>
            <option value="pro">Pro</option>
            <option value="business">Business</option>
          </select>
          <select id="mon-risk" class="inp" onchange="loadAccounts(1)">
            <option value="">Semua Risiko</option>
            <option value="LOW">LOW</option>
            <option value="MEDIUM">MEDIUM</option>
            <option value="HIGH">HIGH</option>
            <option value="DORMANT">DORMANT</option>
          </select>
          <label class="chip-btn" style="display:inline-flex;align-items:center;gap:5px">
            <input type="checkbox" id="mon-online-only" onchange="loadAccounts(1)"> Online Live
          </label>
        </div>
      </div>
      <div style="overflow-x:auto">
        <table>
          <thead>
            <tr>
              <th>Akun &amp; Role</th>
              <th>Tier</th>
              <th>Status / Layar Aktif</th>
              <th>Screen Time</th>
              <th>Aktivitas</th>
              <th>Health &amp; Risk</th>
            </tr>
          </thead>
          <tbody id="mon-accounts-tbody">
            <tr><td colspan="6" style="text-align:center;color:var(--text-muted);padding:22px">Masuk akun Admin di atas untuk memuat daftar pengguna online &amp; aktivitas lengkap.</td></tr>
          </tbody>
        </table>
      </div>
      <div style="display:flex;justify-content:space-between;align-items:center;margin-top:14px;font-size:12px;color:var(--text-muted)">
        <span id="mon-page-info">Halaman 1</span>
        <div style="display:flex;gap:8px">
          <button class="btn" onclick="changeMonPage(-1)">◀ Prev</button>
          <button class="btn" onclick="changeMonPage(1)">Next ▶</button>
        </div>
      </div>
    </div>

    <div style="display:flex;flex-direction:column;gap:18px">
      <div class="card">
        <div class="section-head">
          <h2>📊 Adopsi Fitur 7 Hari</h2>
          <span class="badge espresso">RoaringBitmap</span>
        </div>
        <div id="mon-adoption-list"><div style="font-size:12px;color:var(--text-muted)">Menunggu event telemetri fitur...</div></div>
      </div>

      <div class="card">
        <div class="section-head">
          <h2>☕ Top Resep Diseduh</h2>
          <span class="badge amber">30 Hari</span>
        </div>
        <div id="mon-top-recipes"><div style="font-size:12px;color:var(--text-muted)">Belum ada eksperimen seduh 30H.</div></div>
        <div class="section-head" style="margin-top:18px"><h2>🔁 Cohort Retention</h2></div>
        <div id="mon-cohorts"><div style="font-size:12px;color:var(--text-muted)">Terkumpul setelah &gt;1 hari aktif.</div></div>
      </div>
    </div>
  </section>
</div>
"#;

pub const MONITORING_MODAL_HTML: &str = r#"
<div id="mon-dossier-modal" class="modal-bg" onclick="if(event.target===this)closeDossier()">
  <div class="modal-card">
    <div style="width:44px;height:5px;background:#DCD0C2;border-radius:99px;margin:0 auto 14px"></div>
    <div class="section-head">
      <h2 id="dossier-title">360° Dossier Akun</h2>
      <button class="btn" onclick="closeDossier()">✕ Tutup</button>
    </div>
    <div id="dossier-body" style="font-size:12.5px;line-height:1.5">Memuat dossier...</div>
  </div>
</div>
"#;
