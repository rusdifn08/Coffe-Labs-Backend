use crate::modules::telemetry::dashboard_script::DASHBOARD_JS;

pub fn render_dashboard_html() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="id">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Coffee Labs — Enterprise Rust Axum APM & Telemetry Dashboard</title>
<style>
  :root {{
    --bg: #0b0908; --panel: rgba(24, 19, 16, 0.88); --panel-border: rgba(212, 163, 115, 0.18);
    --amber: #d4a373; --gold: #f4a261; --green: #2dd4bf; --blue: #38bdf8; --red: #fb7185;
    --text: #f8f5f2; --muted: #a89f96;
  }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    background: radial-gradient(circle at 15% 0%, #231812 0%, #0b0908 65%);
    color: var(--text); min-height: 100vh; padding: 24px 32px;
  }}
  .header {{
    display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center;
    gap: 16px; padding-bottom: 20px; border-bottom: 1px solid var(--panel-border); margin-bottom: 24px;
  }}
  .brand {{ display: flex; align-items: center; gap: 14px; }}
  .logo {{
    width: 46px; height: 46px; border-radius: 14px;
    background: linear-gradient(135deg, #d4a373, #8d5b4c);
    display: flex; align-items: center; justify-content: center; font-size: 24px;
    box-shadow: 0 8px 24px rgba(212, 163, 115, 0.25);
  }}
  .title h1 {{ font-size: 20px; font-weight: 800; letter-spacing: -0.02em; }}
  .title p {{ font-size: 12px; color: var(--muted); margin-top: 3px; }}
  .actions {{ display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }}
  .btn {{
    background: rgba(212, 163, 115, 0.12); color: var(--amber);
    border: 1px solid var(--panel-border); padding: 9px 15px; border-radius: 10px;
    font-size: 12px; font-weight: 600; cursor: pointer; transition: all 0.2s; text-decoration: none;
  }}
  .btn:hover {{ background: rgba(212, 163, 115, 0.24); transform: translateY(-1px); }}
  .btn.primary {{ background: linear-gradient(135deg, #d4a373, #b07d4f); color: #120d0a; border: none; }}
  .grid-4 {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 16px; margin-bottom: 24px; }}
  .card {{
    background: var(--panel); border: 1px solid var(--panel-border);
    border-radius: 16px; padding: 18px 20px; backdrop-filter: blur(12px);
  }}
  .card-label {{ font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }}
  .card-val {{ font-size: 28px; font-weight: 800; margin: 8px 0 4px; color: var(--text); }}
  .card-sub {{ font-size: 12px; color: var(--muted); display: flex; justify-content: space-between; }}
  .badge {{
    display: inline-flex; align-items: center; gap: 5px;
    padding: 3px 9px; border-radius: 99px; font-size: 11px; font-weight: 700;
  }}
  .badge.ok {{ background: rgba(45, 212, 191, 0.14); color: var(--green); }}
  .badge.warn {{ background: rgba(244, 162, 97, 0.16); color: var(--gold); }}
  .badge.err {{ background: rgba(251, 113, 133, 0.16); color: var(--red); }}
  . badge-method {{
    padding: 3px 8px; border-radius: 6px; font-size: 10px; font-weight: 800; font-family: monospace;
  }}
  .GET {{ background: rgba(56, 189, 248, 0.15); color: var(--blue); }}
  .POST {{ background: rgba(45, 212, 191, 0.15); color: var(--green); }}
  .PUT {{ background: rgba(244, 162, 97, 0.15); color: var(--gold); }}
  .DELETE {{ background: rgba(251, 113, 133, 0.15); color: var(--red); }}
  .split {{ display: grid; grid-template-columns: 2fr 1fr; gap: 20px; margin-bottom: 24px; }}
  @media (max-width: 1100px) {{ .split {{ grid-template-columns: 1fr; }} }}
  .section-head {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; }}
  .section-head h2 {{ font-size: 15px; font-weight: 700; color: var(--amber); }}
  table {{ width: 100%; border-collapse: collapse; font-size: 12.5px; }}
  th {{ text-align: left; color: var(--muted); font-weight: 600; padding: 10px 8px; border-bottom: 1px solid var(--panel-border); font-size: 11px; text-transform: uppercase; }}
  td {{ padding: 10px 8px; border-bottom: 1px solid rgba(255,255,255,0.04); font-family: 'JetBrains Mono', monospace; }}
  .bar-wrap {{ width: 90px; height: 6px; background: rgba(255,255,255,0.07); border-radius: 99px; overflow: hidden; }}
  .bar-fill {{ height: 100%; background: linear-gradient(90deg, var(--green), var(--amber)); border-radius: 99px; }}
  .counts-grid {{ display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-top: 12px; }}
  .mini-stat {{ background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); padding: 10px; border-radius: 10px; text-align: center; }}
  .mini-stat b {{ display: block; font-size: 18px; color: var(--amber); }}
  .mini-stat span {{ font-size: 10.5px; color: var(--muted); }}
  .trace-list {{ max-height: 340px; overflow-y: auto; display: flex; flex-direction: column; gap: 8px; }}
  .trace-item {{
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; border-radius: 10px; background: rgba(255,255,255,0.025);
    font-family: monospace; font-size: 12px;
  }}
</style>
</head>
<body>
  <header class="header">
    <div class="brand">
      <div class="logo">☕</div>
      <div class="title">
        <h1>Coffee Labs — Rust Axum Enterprise Telemetry & Performance Center</h1>
        <p id="sub-banner">mimalloc Global Allocator • Ed25519 Asymmetric JWT • NeonDB Serverless Pool • Zero-Trust RBAC</p>
      </div>
    </div>
    <div class="actions">
      <span id="conn-badge" class="badge ok">● LIVE STREAM (2s)</span>
      <button class="btn primary" onclick="runApiBenchmark()">⚡ Benchmark 8 Mobile Endpoints</button>
      <button class="btn" onclick="fetchTelemetry()">🔄 Refresh</button>
      <button class="btn" onclick="resetTelemetry()">🧹 Reset Metrics</button>
      <a class="btn" href="/api/v1/openapi.json" target="_blank">📄 OpenAPI 3.1 JSON</a>
    </div>
  </header>

  <section class="grid-4">
    <div class="card">
      <div class="card-label">Total API Requests & Throughput</div>
      <div class="card-val" id="kpi-total">0</div>
      <div class="card-sub">
        <span>Throughput: <b id="kpi-rps">0.00</b> req/s</span>
        <span>In-Flight: <b id="kpi-inflight">0</b></span>
      </div>
    </div>
    <div class="card">
      <div class="card-label">Global API Response Latency</div>
      <div class="card-val" id="kpi-avg-ms">0.00 ms</div>
      <div class="card-sub">
        <span>P95: <b id="kpi-p95-ms">0.00 ms</b></span>
        <span>P99: <b id="kpi-p99-ms">0.00 ms</b></span>
      </div>
    </div>
    <div class="card">
      <div class="card-label">NeonDB Serverless Pool Health</div>
      <div class="card-val" id="kpi-db-ping">0.00 ms</div>
      <div class="card-sub">
        <span id="kpi-db-pool">Pool: 0 / 15</span>
        <span id="kpi-db-idle">Idle: 0</span>
      </div>
    </div>
    <div class="card">
      <div class="card-label">Security & HTTP Status Distribution</div>
      <div class="card-val" id="kpi-2xx">0 OK</div>
      <div class="card-sub">
        <span>4xx Guard: <b id="kpi-4xx">0</b></span>
        <span>5xx Err: <b id="kpi-5xx">0</b> (<span id="kpi-err-rate">0%</span>)</span>
      </div>
    </div>
  </section>

  <section class="split">
    <div class="card">
      <div class="section-head">
        <h2>⚡ Per-Endpoint API Response Time & Percentile Telemetry (P50 / P95 / P99)</h2>
        <span id="uptime-label" style="font-size:12px;color:var(--muted)">Uptime: 0s</span>
      </div>
      <div style="overflow-x:auto">
        <table>
          <thead>
            <tr>
              <th>Method</th><th>Endpoint Route</th><th>Hits</th>
              <th>Min</th><th>Avg</th><th>P50</th><th>P95</th><th>P99</th><th>Max</th>
              <th>Visual</th><th>Status</th>
            </tr>
          </thead>
          <tbody id="endpoints-tbody"></tbody>
        </table>
      </div>
    </div>

    <div class="card">
      <div class="section-head">
        <h2>🗄️ Live NeonDB Domain Records</h2>
        <span class="badge ok">Synced</span>
      </div>
      <div class="counts-grid" id="domain-counts"></div>
      <div class="section-head" style="margin-top:20px">
        <h2>📡 Live Request Trace Stream</h2>
      </div>
      <div class="trace-list" id="trace-stream"></div>
    </div>
  </section>

  <section class="card">
    <div class="section-head">
      <h2>🧩 Modular Backend Services & Mobile Feature Architecture (10 Domains)</h2>
      <span class="badge ok">All Services Operational</span>
    </div>
    <div style="overflow-x:auto">
      <table>
        <thead>
          <tr><th>Domain Module</th><th>Service Name</th><th>Base Route</th><th>Mobile Feature Integration</th><th>Security & Guard Layer</th><th>State</th></tr>
        </thead>
        <tbody id="services-tbody"></tbody>
      </table>
    </div>
  </section>

  <script>{}</script>
</body>
</html>"#,
        DASHBOARD_JS
    )
}
