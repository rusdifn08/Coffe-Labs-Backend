pub const ADMIN_INFRA_JS: &str = r#"
let adminUsersCache = [];
let adminStatusFilter = 'ALL';

function setAdminStatusFilter(status, btn) {
  adminStatusFilter = status;
  document.querySelectorAll('#tab-admin .chip-btn').forEach(b => b.classList.remove('active'));
  if (btn) btn.classList.add('active');
  renderAdminUsersTable();
}

async function loadAdminUsersPanel() {
  const token = getMonToken();
  if (!token) return;
  const headers = { Authorization: 'Bearer ' + token };
  try {
    const [meRes, statsRes, usersRes] = await Promise.all([
      fetch('/api/v1/auth/me', { headers }),
      fetch('/api/admin/users/stats', { headers }),
      fetch('/api/admin/users', { headers }),
    ]);
    if (meRes.ok) {
      const me = (await meRes.json()).data;
      if (me) {
        document.getElementById('mon-me-pill').textContent = '@' + (me.username || 'admin');
        document.getElementById('adm-me-badge').textContent = (me.role || 'user').toUpperCase() + ' • ' + (me.tier || 'trial').toUpperCase();
        document.getElementById('adm-me-badge').className = 'badge ok';
        document.getElementById('adm-me-body').innerHTML =
          `<b>@${me.username}</b> (${me.full_name || '-'}) • Role: <span class="badge espresso">${me.role}</span> • Tier: <span class="badge warn">${me.tier}</span> • Status: <span class="badge ok">${me.status}</span>`;
      }
    }
    if (statsRes.ok) {
      const s = (await statsRes.json()).data;
      if (s) {
        document.getElementById('adm-total-users').textContent = s.total_users ?? 0;
        document.getElementById('adm-new-7d').textContent = s.new_registrations_7d ?? 0;
        document.getElementById('adm-active-users').textContent = s.active_users ?? 0;
        document.getElementById('adm-pending-users').textContent = s.pending_or_declined ?? 0;
        document.getElementById('nav-badge-users').textContent = `${s.total_users ?? 0} User`;
      }
    }
    if (usersRes.ok) {
      adminUsersCache = (await usersRes.json()).data || [];
      renderAdminUsersTable();
    }
  } catch (_) {}
}

function renderAdminUsersTable() {
  const tbody = document.getElementById('adm-users-tbody');
  if (!tbody) return;
  const q = (document.getElementById('adm-search')?.value || '').trim().toLowerCase();
  const filtered = adminUsersCache.filter(u => {
    if (adminStatusFilter !== 'ALL' && u.status !== adminStatusFilter) return false;
    if (q && !(u.username || '').toLowerCase().includes(q) && !(u.full_name || '').toLowerCase().includes(q)) return false;
    return true;
  });
  if (!filtered.length) {
    tbody.innerHTML = '<tr><td colspan="5" style="text-align:center;color:var(--text-muted);padding:20px">Belum ada data pengguna cocok.</td></tr>';
    return;
  }
  tbody.innerHTML = filtered.map(u => {
    const ch = (u.username || '?')[0].toUpperCase();
    const stCls = u.status === 'approved' ? 'ok' : (u.status === 'declined' ? 'err' : 'warn');
    const isAdmin = u.role === 'admin';
    return `<tr>
      <td><div style="display:flex;align-items:center;gap:10px;font-family:'Poppins',sans-serif">
        <div class="avatar-circle">${ch}</div>
        <div><b>@${u.username}</b><div style="font-size:10.5px;color:var(--text-muted)">${u.full_name || '-'}</div></div>
      </div></td>
      <td><span class="badge espresso">${u.role}</span> <span class="badge ${stCls}">${u.status}</span></td>
      <td><select class="inp" style="padding:4px 8px;font-size:11px" ${isAdmin ? 'disabled' : ''} onchange="adminSetTier('${u.id}', this.value)">
        <option value="trial" ${u.tier === 'trial' ? 'selected' : ''}>Trial</option>
        <option value="pro" ${u.tier === 'pro' ? 'selected' : ''}>Pro</option>
        <option value="business" ${u.tier === 'business' ? 'selected' : ''}>Business</option>
      </select></td>
      <td style="color:var(--text-muted);font-size:11px">${u.created_at ? new Date(u.created_at).toLocaleDateString() : '-'}</td>
      <td><div style="display:flex;gap:6px;flex-wrap:wrap">
        <button class="btn" style="padding:4px 9px;font-size:11px" onclick="adminUserAction('${u.id}','accept')">✅ Approve</button>
        <button class="btn" style="padding:4px 9px;font-size:11px" onclick="adminUserAction('${u.id}','decline')">⛔ Decline</button>
        <button class="btn" style="padding:4px 9px;font-size:11px" onclick="openDossier('${u.id}')">🔍 Dossier</button>
      </div></td>
    </tr>`;
  }).join('');
}

async function adminUserAction(id, act) {
  const token = getMonToken();
  if (!token) return;
  await fetch(`/api/admin/users/${id}/${act}`, { method: 'POST', headers: { Authorization: 'Bearer ' + token } });
  await loadAdminUsersPanel();
}

async function adminSetTier(id, tier) {
  const token = getMonToken();
  if (!token) return;
  await fetch(`/api/admin/users/${id}/tier`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json', Authorization: 'Bearer ' + token },
    body: JSON.stringify({ tier })
  });
  await loadAdminUsersPanel();
}

async function loadInfraHealth() {
  try {
    const res = await fetch('/api/health');
    if (!res.ok) return;
    const h = await res.json();
    document.getElementById('inf-db-status').textContent = h.db_ok ? 'CONNECTED' : 'DEGRADED';
    document.getElementById('inf-db-meta').textContent = h.security || 'NeonDB Rustls';
    const r = h.redis || {};
    document.getElementById('inf-redis-status').textContent = r.connected ? 'CONNECTED' : (r.configured ? 'STANDBY' : 'LOCAL MEM');
    document.getElementById('inf-redis-meta').textContent = `Pool: ${r.pool_size ?? 0} • MaxLen: ${r.stream_maxlen ?? 0}`;
    const mq = h.rabbitmq || {};
    document.getElementById('inf-mq-status').textContent = mq.connected ? 'CONNECTED' : (mq.configured ? 'STANDBY' : 'LOCAL BUS');
    document.getElementById('inf-mq-meta').textContent = `Exchange: ${mq.exchange || '-'} • Prefetch: ${mq.prefetch_count ?? 0}`;
    const c = h.caches || {};
    const totalCache = (c.profile_l1_entries ?? 0) + (c.chat_member_l1_entries ?? 0) + (c.fcm_token_l1_entries ?? 0);
    document.getElementById('inf-cache-entries').textContent = `${totalCache} Entries`;
    document.getElementById('inf-fcm-cloud').textContent = `FCM: ${h.fcm_configured ? 'Active' : 'Off'} • Cloudinary: ${h.cloudinary_configured ? 'Active' : 'Local'}`;
  } catch (_) {}
}

function applyServicesFilter() {
  const tbody = document.getElementById('services-tbody');
  if (!tbody) return;
  const q = (document.getElementById('svc-search')?.value || '').trim().toLowerCase();
  const list = (window.__servicesCache || []).filter(s =>
    !q || s.module.toLowerCase().includes(q) || s.name.toLowerCase().includes(q) || s.base_path.toLowerCase().includes(q)
  );
  tbody.innerHTML = list.map(s => `<tr>
    <td><span class="badge espresso">${s.module}</span></td>
    <td style="font-family:'Poppins',sans-serif;font-weight:700;color:var(--text-dark)">${s.name}</td>
    <td style="color:var(--roast);font-weight:600">${s.base_path}</td>
    <td style="font-family:'Poppins',sans-serif;color:var(--text-muted)">${s.mobile_feature}</td>
    <td><span class="badge warn">${s.security_tier}</span></td>
    <td><span class="badge ok">● ${s.status}</span></td>
  </tr>`).join('');
}

loadAdminUsersPanel();
loadInfraHealth();
setInterval(() => {
  if (!document.getElementById('tab-services').classList.contains('hidden')) loadInfraHealth();
}, 10000);
"#;
