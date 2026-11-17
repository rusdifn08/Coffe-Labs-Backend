pub const DASHBOARD_JS: &str = r#"
let apmEndpointsCache = [];
let apmStatusFilter = 'ALL';

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

    const up = formatUptime(d.uptime_seconds);
    document.getElementById('uptime-label').textContent = 'Uptime: ' + up;
    document.getElementById('nav-uptime-pill').textContent = '⏱️ ' + up;
    document.getElementById('nav-badge-reqs').textContent = d.total_requests.toLocaleString();
    const errTotal = (d.status_4xx || 0) + (d.status_5xx || 0);
    const errPill = document.getElementById('nav-err-pill');
    errPill.textContent = errTotal > 0 ? `Guard/Err: ${errTotal}` : '0 Errors';
    errPill.className = errTotal > 0 ? 'badge warn' : 'badge ok';

    apmEndpointsCache = d.endpoints || [];
    window.__servicesCache = d.services || [];
    document.getElementById('nav-badge-services').textContent = (window.__servicesCache.length || 0) + ' Modul';

    renderWatchlist(apmEndpointsCache);
    applyApmFilters();
    renderDomainCounts(d.domain_counts || {}, d.monitoring_summary?.online_now ?? 0);
    renderTraces(d.recent_traces || []);
    if (typeof applyServicesFilter === 'function') applyServicesFilter();
    if (typeof renderPublicSummary === 'function' && d.monitoring_summary && !sessionStorage.getItem('coffeelabs_mon_jwt')) {
      renderPublicSummary(d.monitoring_summary);
    }

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

function routeFeatureTag(path) {
  if (path.includes('/monitoring/ingest')) return 'Telemetry Batch';
  if (path.includes('/monitoring/heartbeat')) return 'User Online';
  if (path.includes('/monitoring')) return 'Monitoring Hub';
  if (path.includes('/admin/users')) return 'Admin Users';
  if (path.includes('/auth/')) return 'Auth & Session';
  if (path.includes('/toolkit/')) return 'Barista LMS';
  if (path.includes('/chat/ws')) return 'Chat Socket';
  if (path.includes('/chat/')) return 'Chat API';
  if (path.includes('/social')) return 'Social Feed';
  if (path.includes('/beans')) return 'Beans & AI';
  return 'Mobile API';
}

function setApmStatusFilter(mode, btn) {
  apmStatusFilter = mode;
  document.querySelectorAll('#apm-status-chips .chip-btn').forEach(b => b.classList.remove('active'));
  if (btn) btn.classList.add('active');
  applyApmFilters();
}

function applyApmFilters() {
  const q = (document.getElementById('apm-search')?.value || '').trim().toLowerCase();
  const methodF = document.getElementById('apm-method-filter')?.value || 'ALL';
  const filtered = apmEndpointsCache.filter(e => {
    if (q && !e.path.toLowerCase().includes(q) && !routeFeatureTag(e.path).toLowerCase().includes(q)) return false;
    if (methodF !== 'ALL' && e.method !== methodF) return false;
    if (apmStatusFilter === '2XX' && (e.last_status < 200 || e.last_status >= 300)) return false;
    if (apmStatusFilter === 'ERR' && e.last_status < 400) return false;
    if (apmStatusFilter === 'WS' && e.last_status !== 101 && !e.path.includes('/ws')) return false;
    return true;
  });
  renderEndpoints(filtered);
}

async function runApiBenchmark() {
  const endpoints = [
    ['/api/v1/health/redis', { method: 'GET' }],
    ['/api/v1/health/rabbitmq', { method: 'GET' }],
    ['/api/v1/rbac/plans', { method: 'GET' }],
    ['/api/v1/toolkit/lms/courses', { method: 'GET' }],
    ['/api/v1/social/posts?sort=latest&limit=10', { method: 'GET' }],
    ['/api/v1/social/feed/cursor?limit=5', { method: 'GET' }],
    ['/api/beans', { method: 'GET' }],
    ['/api/beans/dial-in', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
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

function refreshAllPanels() {
  fetchTelemetry();
  if (typeof refreshMonitoring === 'function') refreshMonitoring();
  if (typeof loadAdminUsersPanel === 'function') loadAdminUsersPanel();
  if (typeof loadInfraHealth === 'function') loadInfraHealth();
}

fetchTelemetry();
setInterval(fetchTelemetry, 2000);
"#;
