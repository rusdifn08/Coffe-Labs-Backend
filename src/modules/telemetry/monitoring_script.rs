pub const MONITORING_JS: &str = r#"
let monPage = 1;
let monDebounce = null;
const MON_JWT_KEY = 'coffeelabs_mon_jwt';

function switchDashTab(tab, pushUrl) {
  const valid = ['apm', 'monitoring', 'admin', 'services'];
  const target = valid.includes(tab) ? tab : 'apm';
  document.querySelectorAll('.nav-btn').forEach(b => b.classList.toggle('active', b.dataset.tab === target));
  valid.forEach(t => {
    const el = document.getElementById('tab-' + t);
    if (el) el.classList.toggle('hidden', t !== target);
  });
  if (pushUrl && window.history && window.history.replaceState) {
    const url = target === 'apm' ? '/dashboard/apm' : '/dashboard/' + target;
    window.history.replaceState(null, '', url);
  }
  if (target === 'monitoring') refreshMonitoring();
  if (target === 'admin' && typeof loadAdminUsersPanel === 'function') loadAdminUsersPanel();
  if (target === 'services' && typeof loadInfraHealth === 'function') loadInfraHealth();
}

function initActiveTab() {
  const p = window.location.pathname || '';
  const q = new URLSearchParams(window.location.search).get('tab');
  let initial = document.body.dataset.defaultTab || 'apm';
  if (q) initial = q;
  else if (p.includes('/monitoring')) initial = 'monitoring';
  else if (p.includes('/admin')) initial = 'admin';
  else if (p.includes('/services')) initial = 'services';
  else if (p.includes('/apm')) initial = 'apm';
  switchDashTab(initial, false);
}

function fmtDur(sec) {
  const s = Math.max(0, Math.round(sec || 0));
  if (s < 60) return s + 's';
  const m = Math.floor(s / 60);
  if (m < 60) return m + 'm ' + (s % 60) + 's';
  return Math.floor(m / 60) + 'j ' + (m % 60) + 'm';
}

function getMonToken() { return sessionStorage.getItem(MON_JWT_KEY); }

async function monLogin() {
  const username = document.getElementById('mon-user').value.trim();
  const password = document.getElementById('mon-pass').value;
  if (!username || !password) return;
  try {
    const res = await fetch('/api/v1/auth/login', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password })
    });
    const j = await res.json();
    const token = j?.data?.access_token;
    if (!res.ok || !token) throw new Error(j.message || 'Login gagal');
    sessionStorage.setItem(MON_JWT_KEY, token);
    syncMonAuthUi();
    await refreshMonitoring();
    if (typeof loadAdminUsersPanel === 'function') await loadAdminUsersPanel();
  } catch (e) {
    document.getElementById('mon-auth-status').textContent = '❌ ' + e.message;
  }
}

function monLogout() {
  sessionStorage.removeItem(MON_JWT_KEY);
  syncMonAuthUi();
  document.getElementById('mon-accounts-tbody').innerHTML =
    '<tr><td colspan="6" style="text-align:center;color:var(--text-muted);padding:20px">Masuk Admin di atas untuk melihat daftar lengkap semua user.</td></tr>';
  document.getElementById('mon-online-spotlight').innerHTML =
    '<div style="font-size:12px;color:var(--text-muted)">Masuk Admin di atas untuk melihat pengguna yang sedang online.</div>';
}

function syncMonAuthUi() {
  const has = !!getMonToken();
  document.getElementById('mon-login-controls').style.display = has ? 'none' : 'flex';
  document.getElementById('mon-unlocked-controls').style.display = has ? 'flex' : 'none';
  if (has) document.getElementById('mon-auth-status').textContent = '✅ Sesi Admin/Business aktif — User Online, 360° Dossier & /api/admin/users terbuka.';
}

async function refreshMonitoring() {
  const token = getMonToken();
  if (!token) return;
  try {
    const res = await fetch('/api/v1/monitoring/overview', { headers: { Authorization: 'Bearer ' + token } });
    if (res.status === 401 || res.status === 403) { monLogout(); return; }
    const o = (await res.json()).data;
    if (o) {
      renderPublicSummary(o);
      document.getElementById('mon-tier-split').textContent = `T:${o.trial_accounts} P:${o.pro_accounts} B:${o.business_accounts}`;
      document.getElementById('mon-avg-screen').textContent = fmtDur(o.avg_today_screen_time_sec);
      document.getElementById('mon-brews-7d').textContent = o.total_brews_7d;
      document.getElementById('mon-churn-count').textContent = (o.at_risk_high_count || 0) + (o.dormant_count || 0);
      document.getElementById('mon-risk-high').textContent = o.at_risk_high_count || 0;
      document.getElementById('mon-dormant').textContent = o.dormant_count || 0;
      renderCohorts(o.cohorts || []);
    }
    await loadAccounts(monPage);
  } catch (_) {}
}

function debouncedLoadAccounts() { clearTimeout(monDebounce); monDebounce = setTimeout(() => loadAccounts(1), 260); }
function changeMonPage(delta) { loadAccounts(Math.max(1, monPage + delta)); }

async function loadAccounts(page) {
  const token = getMonToken();
  if (!token) return;
  monPage = page;
  const params = new URLSearchParams({
    page: String(page), per_page: '25',
    q: document.getElementById('mon-q').value.trim(),
    tier: document.getElementById('mon-tier').value,
    risk: document.getElementById('mon-risk').value,
    online_only: String(document.getElementById('mon-online-only').checked),
  });
  const res = await fetch('/api/v1/monitoring/accounts?' + params.toString(), { headers: { Authorization: 'Bearer ' + token } });
  if (!res.ok) return;
  const d = (await res.json()).data;
  renderOnlineSpotlight(d.items || []);
  const tbody = document.getElementById('mon-accounts-tbody');
  document.getElementById('mon-page-info').textContent = `Hal ${d.page} • Total ${d.total} akun (${d.online_now} Online)`;
  if (!d.items.length) { tbody.innerHTML = '<tr><td colspan="6" style="text-align:center;color:var(--text-muted);padding:20px">Tidak ada akun cocok.</td></tr>'; return; }
  tbody.innerHTML = d.items.map(u => {
    const ch = (u.username || '?')[0].toUpperCase();
    return `<tr class="clickable-row" onclick="openDossier('${u.user_id}')">
      <td><div style="display:flex;align-items:center;gap:10px;font-family:'Poppins',sans-serif">
        <div class="avatar-circle">${ch}<span class="dot-badge ${u.is_online ? 'on' : 'off'}"></span></div>
        <div><b>@${u.username}</b><div style="font-size:10.5px;color:var(--text-muted)">${u.full_name} (${u.role})</div></div>
      </div></td>
      <td><span class="badge espresso">${u.account_tier}</span></td>
      <td>${u.is_online ? `<span class="badge ok">LIVE</span> <b>${u.current_active_route || '/'}</b>` : (u.last_seen_at ? new Date(u.last_seen_at).toLocaleString() : 'Belum aktif')}<div style="font-size:10.5px;color:var(--text-muted)">${u.current_feature_bucket || '-'}</div></td>
      <td>Hari Ini: <b style="color:var(--roast)">${fmtDur(u.today_screen_time_sec)}</b><div style="font-size:10.5px;color:var(--text-muted)">7H: ${fmtDur(u.total_7d_screen_time_sec)} • All: ${fmtDur(u.total_lifetime_screen_time_sec)}</div></td>
      <td>☕ ${u.total_brews_executed} • 📝 ${u.total_social_posts}</td>
      <td><b>${u.health_score}/100</b> <span class="pill ${u.churn_risk_level}">${u.churn_risk_level}</span></td>
    </tr>`;
  }).join('');
}

async function openDossier(userId) {
  const token = getMonToken();
  if (!token) return;
  document.getElementById('mon-dossier-modal').classList.add('open');
  document.getElementById('dossier-body').textContent = 'Memuat profil 360°...';
  const res = await fetch('/api/v1/monitoring/accounts/' + userId + '/dossier', { headers: { Authorization: 'Bearer ' + token } });
  const d = (await res.json()).data;
  if (!d) return;
  const a = d.account;
  document.getElementById('dossier-title').textContent = `360° Dossier — @${a.username} (${a.full_name})`;
  const feats = (d.feature_breakdown || []).map(f =>
    `<div class="feat-row"><span style="flex:1">${f.label}</span><div class="bar-wrap"><div class="bar-fill" style="width:${Math.min(100, Math.round(f.share_percent))}%"></div></div><b>${fmtDur(f.seconds)} (${f.share_percent}%)</b></div>`).join('') || '<div style="color:var(--text-muted)">Belum ada rincian fitur.</div>';
  const brews = (d.recent_brew_experiments || []).map(b =>
    `<li style="margin:4px 0">☕ <b>${b.recipe_title}</b> (${b.brew_method}) — ${fmtDur(b.brew_duration_sec)} ${b.completed ? '✅' : '⏸️'}</li>`).join('') || '<li>Belum ada log seduh terekam.</li>';
  document.getElementById('dossier-body').innerHTML = `
    <div class="grid-4" style="margin-bottom:14px">
      <div class="mini-stat"><b>${a.health_score}</b><span>Health (${a.churn_risk_level})</span></div>
      <div class="mini-stat"><b>${fmtDur(a.today_screen_time_sec)}</b><span>Hari Ini</span></div>
      <div class="mini-stat"><b>${fmtDur(a.total_7d_screen_time_sec)}</b><span>7 Hari</span></div>
      <div class="mini-stat"><b>${a.total_brews_executed}</b><span>Total Seduh</span></div>
    </div>
    <div class="card" style="margin-bottom:12px"><h4 style="color:var(--roast);margin-bottom:6px">Proporsi Waktu Fitur</h4>${feats}</div>
    <div class="card"><h4 style="color:var(--roast);margin-bottom:6px">Eksperimen Seduh Terakhir</h4><ul style="padding-left:18px">${brews}</ul></div>`;
}
function closeDossier() { document.getElementById('mon-dossier-modal').classList.remove('open'); }
syncMonAuthUi();
initActiveTab();
setInterval(refreshMonitoring, 5000);
"#;
