pub const DASHBOARD_JS: &str = r#"
async function fetchTelemetry() {
  try {
    const res = await fetch('/api/v1/telemetry/metrics');
    if (!res.ok) throw new Error('HTTP ' + res.status);
    const json = await res.json();
    const d = json.data;
    if (!d) return;

    document.getElementById('kpi-total').textContent = d.total_requests.toLocaleString();
    document.getElementById('kpi-rps').textContent = d.requests_per_second.toFixed(2);
    document.getElementById('kpi-inflight').textContent = d.in_flight_requests;
    document.getElementById('kpi-avg-ms').textContent = d.global_avg_latency_ms.toFixed(2) + ' ms';
    document.getElementById('kpi-p95-ms').textContent = d.global_p95_latency_ms.toFixed(2) + ' ms';
    document.getElementById('kpi-p99-ms').textContent = d.global_p99_latency_ms.toFixed(2) + ' ms';

    document.getElementById('kpi-db-ping').textContent = d.database.ping_latency_ms.toFixed(2) + ' ms';
    document.getElementById('kpi-db-pool').textContent = `Pool: ${d.database.pool_size} / ${d.database.max_connections}`;
    document.getElementById('kpi-db-idle').textContent = `Idle: ${d.database.idle_connections}`;

    document.getElementById('kpi-2xx').textContent = d.status_2xx.toLocaleString() + ' (2xx)';
    document.getElementById('kpi-4xx').textContent = d.status_4xx;
    document.getElementById('kpi-5xx').textContent = d.status_5xx;
    document.getElementById('kpi-err-rate').textContent = d.error_rate_percent.toFixed(1) + '%';
    document.getElementById('uptime-label').textContent = `Uptime: ${formatUptime(d.uptime_seconds)}`;

    renderEndpoints(d.endpoints || []);
    renderDomainCounts(d.domain_counts || {});
    renderTraces(d.recent_traces || []);
    renderServices(d.services || []);

    const badge = document.getElementById('conn-badge');
    badge.className = 'badge ok';
    badge.textContent = '● LIVE STREAM (2s)';
  } catch (err) {
    const badge = document.getElementById('conn-badge');
    badge.className = 'badge err';
    badge.textContent = '● RECONNECTING...';
  }
}

function formatUptime(sec) {
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = sec % 60;
  return `${h}h ${m}m ${s}s`;
}

function renderEndpoints(list) {
  const tbody = document.getElementById('endpoints-tbody');
  if (!list.length) {
    tbody.innerHTML = '<tr><td colspan="11" style="text-align:center;color:var(--muted)">Belum ada trafik API terekam. Klik "Benchmark 8 Mobile Endpoints" di atas.</td></tr>';
    return;
  }
  const maxAvg = Math.max(...list.map(e => e.avg_ms), 10);
  tbody.innerHTML = list.map(e => {
    const pct = Math.min(100, Math.max(6, Math.round((e.avg_ms / maxAvg) * 100)));
    const stClass = e.last_status >= 500 ? 'err' : (e.last_status >= 400 ? 'warn' : 'ok');
    return `<tr>
      <td><span class="badge-method ${e.method}">${e.method}</span></td>
      <td style="font-weight:600;color:var(--text)">${e.path}</td>
      <td>${e.hits}</td>
      <td>${e.min_ms.toFixed(2)}ms</td>
      <td style="color:var(--amber);font-weight:700">${e.avg_ms.toFixed(2)}ms</td>
      <td>${e.p50_ms.toFixed(2)}ms</td>
      <td>${e.p95_ms.toFixed(2)}ms</td>
      <td>${e.p99_ms.toFixed(2)}ms</td>
      <td>${e.max_ms.toFixed(2)}ms</td>
      <td><div class="bar-wrap"><div class="bar-fill" style="width:${pct}%"></div></div></td>
      <td><span class="badge ${stClass}">${e.last_status}</span></td>
    </tr>`;
  }).join('');
}

function renderDomainCounts(c) {
  const items = [
    ['Users', c.users ?? 0],
    ['Sessions', c.active_sessions ?? 0],
    ['Social Posts', c.social_posts ?? 0],
    ['Comments', c.social_comments ?? 0],
    ['Cloud Recipes', c.synced_recipes ?? 0],
    ['Brew Logs', c.brew_logs ?? 0],
    ['Curated Beans', c.curated_beans ?? 0],
    ['Orders', c.marketplace_orders ?? 0],
    ['Feedbacks', c.feedbacks ?? 0],
  ];
  document.getElementById('domain-counts').innerHTML = items
    .map(([label, val]) => `<div class="mini-stat"><b>${val}</b><span>${label}</span></div>`)
    .join('');
}

function renderTraces(traces) {
  const container = document.getElementById('trace-stream');
  container.innerHTML = traces.slice(0, 15).map(t => {
    const stClass = t.status >= 500 ? 'err' : (t.status >= 400 ? 'warn' : 'ok');
    const timeStr = new Date(t.timestamp).toLocaleTimeString();
    return `<div class="trace-item">
      <div><span class="badge-method ${t.method}">${t.method}</span> <span style="margin-left:6px">${t.path}</span></div>
      <div style="display:flex;gap:8px;align-items:center">
        <span style="color:var(--amber)">${t.duration_ms.toFixed(2)}ms</span>
        <span class="badge ${stClass}">${t.status}</span>
        <span style="color:var(--muted);font-size:10px">${timeStr}</span>
      </div>
    </div>`;
  }).join('');
}

function renderServices(services) {
  const tbody = document.getElementById('services-tbody');
  tbody.innerHTML = services.map(s => `<tr>
    <td style="color:var(--amber);font-weight:700">${s.module}</td>
    <td style="font-family:sans-serif;font-weight:600">${s.name}</td>
    <td>${s.base_path}</td>
    <td style="font-family:sans-serif;color:var(--muted)">${s.mobile_feature}</td>
    <td><span class="badge warn">${s.security_tier}</span></td>
    <td><span class="badge ok">● ${s.status}</span></td>
  </tr>`).join('');
}

async function runApiBenchmark() {
  const endpoints = [
    ['/api/health', { method: 'GET' }],
    ['/api/v1/app/version-check?platform=android&base_version=1.0.0&build_number=1&patch_number=0', { method: 'GET' }],
    ['/api/v1/rbac/plans', { method: 'GET' }],
    ['/api/v1/social/posts?sort=latest&limit=10', { method: 'GET' }],
    ['/api/v1/social/feed/cursor?limit=5', { method: 'GET' }],
    ['/api/beans', { method: 'GET' }],
    ['/api/beans/dial-in', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ bean_name: 'Bali Kintamani Anaerobic', process: 'Anaerobic Natural', coffee_grams: 15 })
    }],
    ['/api/brew-logs/analytics', { method: 'GET' }],
  ];
  await Promise.all(endpoints.map(([url, opts]) => fetch(url, opts).catch(() => {})));
  await fetchTelemetry();
}

async function resetTelemetry() {
  await fetch('/api/v1/telemetry/reset', { method: 'POST' });
  await fetchTelemetry();
}

fetchTelemetry();
setInterval(fetchTelemetry, 2000);
"#;
