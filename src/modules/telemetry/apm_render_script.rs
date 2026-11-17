pub const APM_RENDER_JS: &str = r#"
function renderWatchlist(list) {
  const watchTargets = [
    ['POST', '/api/v1/monitoring/ingest-batch', 'Batch Telemetry', 'monitoring'],
    ['POST', '/api/v1/monitoring/heartbeat', 'User Online Pulse', 'monitoring'],
    ['GET', '/api/admin/users', 'Admin Directory', 'admin'],
    ['GET', '/api/admin/users/stats', 'Admin User Stats', 'admin'],
    ['GET', '/api/v1/auth/me', 'Session Profile', 'admin'],
    ['GET', '/api/v1/toolkit/lms/courses', 'LMS Courses', 'services'],
    ['GET', '/api/v1/chat/ws', 'Live Chat WS', 'services'],
  ];
  const byKey = new Map(list.map(e => [`${e.method} ${e.path}`, e]));
  document.getElementById('apm-watchlist-grid').innerHTML = watchTargets.map(([m, p, lbl, tab]) => {
    const mData = byKey.get(`${m} ${p}`);
    const st = mData ? mData.last_status : null;
    const cls = st === null ? 'espresso' : (st === 101 ? 'ws' : (st >= 500 ? 'err' : (st >= 400 ? 'warn' : 'ok')));
    const stLabel = st === null ? 'Standby' : (st === 101 ? '101 WS' : String(st));
    const sub = mData ? `${mData.hits} hits • ${mData.avg_ms.toFixed(2)}ms avg` : 'Belum ada request sesi ini';
    return `<div class="diag-item" style="cursor:pointer" onclick="switchDashTab('${tab}', true)">
      <div style="min-width:0">
        <div style="display:flex;gap:6px;align-items:center;margin-bottom:3px">
          <span class="badge-method ${m}">${m}</span>
          <b style="font-size:12px;color:var(--roast)">${lbl}</b>
        </div>
        <div style="font-size:11px;font-family:'JetBrains Mono',monospace;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${p}</div>
        <div style="font-size:10.5px;color:var(--text-muted);margin-top:2px">${sub}</div>
      </div>
      <span class="badge ${cls}">${stLabel}</span>
    </div>`;
  }).join('');
}

function renderEndpoints(list) {
  const tbody = document.getElementById('endpoints-tbody');
  if (!list.length) {
    tbody.innerHTML = '<tr><td colspan="11" style="text-align:center;color:var(--text-muted);padding:20px">Belum ada trafik API cocok dengan filter. Klik "Benchmark Mobile API" di atas.</td></tr>';
    return;
  }
  const maxAvg = Math.max(...list.map(e => e.avg_ms), 10);
  tbody.innerHTML = list.map(e => {
    const pct = Math.min(100, Math.max(6, Math.round((e.avg_ms / maxAvg) * 100)));
    const stClass = e.last_status === 101 ? 'ws' : (e.last_status >= 500 ? 'err' : (e.last_status >= 400 ? 'warn' : 'ok'));
    const stLabel = e.last_status === 101 ? '101 WS' : e.last_status;
    return `<tr>
      <td><span class="badge-method ${e.method}">${e.method}</span></td>
      <td style="font-weight:700;color:var(--text-dark)">${e.path}<span class="route-tag">${routeFeatureTag(e.path)}</span></td>
      <td><b>${e.hits}</b></td>
      <td>${e.min_ms.toFixed(2)}ms</td>
      <td style="color:var(--roast);font-weight:800">${e.avg_ms.toFixed(2)}ms</td>
      <td>${e.p50_ms.toFixed(2)}ms</td>
      <td>${e.p95_ms.toFixed(2)}ms</td>
      <td>${e.p99_ms.toFixed(2)}ms</td>
      <td>${e.max_ms.toFixed(2)}ms</td>
      <td><div class="bar-wrap"><div class="bar-fill" style="width:${pct}%"></div></div></td>
      <td><span class="badge ${stClass}">${stLabel}</span></td>
    </tr>`;
  }).join('');
}

function renderDomainCounts(c, onlineNow) {
  const items = [
    ['Online Live', onlineNow],
    ['Users', c.users ?? 0],
    ['Sessions', c.active_sessions ?? 0],
    ['Social Posts', c.social_posts ?? 0],
    ['Comments', c.social_comments ?? 0],
    ['Cloud Recipes', c.synced_recipes ?? 0],
    ['Brew Logs', c.brew_logs ?? 0],
    ['Curated Beans', c.curated_beans ?? 0],
    ['Feedbacks', c.feedbacks ?? 0],
  ];
  document.getElementById('domain-counts').innerHTML = items
    .map(([label, val]) => `<div class="mini-stat"><b>${val}</b><span>${label}</span></div>`)
    .join('');
}

function renderTraces(traces) {
  const container = document.getElementById('trace-stream');
  document.getElementById('trace-count-badge').textContent = Math.min(traces.length, 15) + ' Latest';
  container.innerHTML = traces.slice(0, 15).map(t => {
    const stClass = t.status === 101 ? 'ws' : (t.status >= 500 ? 'err' : (t.status >= 400 ? 'warn' : 'ok'));
    const timeStr = new Date(t.timestamp).toLocaleTimeString();
    return `<div class="trace-item">
      <div style="min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">
        <span class="badge-method ${t.method}">${t.method}</span>
        <span style="margin-left:6px;font-weight:600;color:var(--text-dark)">${t.path}</span>
      </div>
      <div style="display:flex;gap:6px;align-items:center;flex-shrink:0">
        <span style="color:var(--roast);font-weight:700">${t.duration_ms.toFixed(2)}ms</span>
        <span class="badge ${stClass}">${t.status}</span>
        <span style="color:var(--text-muted);font-size:10px">${timeStr}</span>
      </div>
    </div>`;
  }).join('');
}
"#;
