pub const MONITORING_RENDER_JS: &str = r#"
function renderPublicSummary(s) {
  if (!s) return;
  const onlineNow = s.online_now ?? 0;
  document.getElementById('mon-online-now').textContent = onlineNow;
  document.getElementById('nav-badge-online').textContent = `${onlineNow} Online`;
  document.getElementById('mon-total-accs').textContent = s.total_accounts ?? 0;
  document.getElementById('mon-dau-wau-mau').textContent = `${s.dau_today || 0} / ${s.wau_7d || 0} / ${s.mau_30d || 0}`;
  document.getElementById('mon-stickiness').textContent = (s.stickiness_pct || 0).toFixed(1) + '%';
  if (s.ram_guard) {
    document.getElementById('mon-ram-guard').textContent =
      `MPSC ${s.ram_guard.mpsc_pending_batches}/${s.ram_guard.mpsc_queue_capacity} • Bitmaps ${s.ram_guard.bitmap_days_in_ram}d • Drop ${s.ram_guard.dropped_batches_total}`;
  }
  renderAdoptionList(s.feature_adoption || []);
  renderTopRecipes(s.top_brew_recipes || []);
}

function renderAdoptionList(list) {
  const el = document.getElementById('mon-adoption-list');
  if (!list.length) { el.innerHTML = '<div style="font-size:12px;color:var(--text-muted)">Belum ada aktivitas fitur 7H.</div>'; return; }
  el.innerHTML = list.slice(0, 6).map(f => `
    <div class="feat-row">
      <div style="flex:1"><b style="color:var(--text-dark)">${f.label}</b><div style="font-size:10.5px;color:var(--text-muted)">${f.active_users_7d} akun • ${f.total_minutes_7d} mnt</div></div>
      <div class="bar-wrap"><div class="bar-fill" style="width:${Math.min(100, Math.round(f.adoption_percent))}%"></div></div>
      <b style="width:44px;text-align:right;color:var(--roast)">${f.adoption_percent.toFixed(1)}%</b>
    </div>`).join('');
}

function renderTopRecipes(list) {
  const el = document.getElementById('mon-top-recipes');
  if (!list.length) { el.innerHTML = '<div style="font-size:12px;color:var(--text-muted)">Belum ada data seduh terekam.</div>'; return; }
  el.innerHTML = list.slice(0, 5).map(r => `
    <div class="feat-row">
      <span style="flex:1">☕ <b>${r.recipe_title}</b> <small style="color:var(--text-muted)">(${r.brew_method})</small></span>
      <span class="badge ok">${r.total_runs}x (${r.completion_rate_pct}%)</span>
    </div>`).join('');
}

function renderCohorts(cohorts) {
  const el = document.getElementById('mon-cohorts');
  if (!cohorts.length) { el.innerHTML = '<div style="font-size:12px;color:var(--text-muted)">Cohort retention muncul setelah data multi-hari terkumpul.</div>'; return; }
  el.innerHTML = cohorts.map(c => `
    <div class="feat-row" style="font-family:'JetBrains Mono',monospace">
      <span>${c.cohort_date} (${c.cohort_size})</span>
      <span>D1:${c.d1_pct ?? '-'}% • D3:${c.d3_pct ?? '-'}% • D7:${c.d7_pct ?? '-'}%</span>
    </div>`).join('');
}

function renderOnlineSpotlight(items) {
  const onlineList = (items || []).filter(u => u.is_online);
  const badge = document.getElementById('mon-spotlight-badge');
  badge.textContent = `● ${onlineList.length} User Online Sekarang`;
  const box = document.getElementById('mon-online-spotlight');
  if (!onlineList.length) {
    box.innerHTML = '<div style="font-size:12px;color:var(--text-muted);padding:4px">Belum ada heartbeat aktif dalam 90 detik terakhir.</div>';
    return;
  }
  box.innerHTML = onlineList.map(u => {
    const ch = (u.username || '?')[0].toUpperCase();
    const brewTag = u.is_brewing_now ? '<span class="badge warn" style="margin-left:4px">☕ Seduh</span>' : '';
    return `<div class="online-chip" onclick="openDossier('${u.user_id}')">
      <div class="avatar-circle">${ch}<span class="dot-badge on"></span></div>
      <div>
        <div style="font-size:12.5px;font-weight:800;color:var(--text-dark)">@${u.username} ${brewTag}</div>
        <div style="font-size:10.5px;color:var(--text-muted)">${u.current_active_route || '/'} • <b style="color:var(--roast)">${u.account_tier}</b></div>
      </div>
    </div>`;
  }).join('');
}
"#;
