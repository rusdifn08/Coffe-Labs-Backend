pub const APM_TAB_HTML: &str = r#"
<div id="tab-apm" class="tab-pane">
  <section class="grid-4">
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Total API Requests &amp; Throughput</div>
          <div class="card-val" id="kpi-total">0</div>
        </div>
        <div class="kpi-icon espresso">⚡</div>
      </div>
      <div class="card-sub">
        <span>Throughput: <b id="kpi-rps">0.00</b> req/s</span>
        <span>In-Flight: <b id="kpi-inflight">0</b></span>
      </div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Global API Response Latency</div>
          <div class="card-val" id="kpi-avg-ms">0.00 ms</div>
        </div>
        <div class="kpi-icon amber">⏱️</div>
      </div>
      <div class="card-sub">
        <span>P95: <b id="kpi-p95-ms">0.00 ms</b></span>
        <span>P99: <b id="kpi-p99-ms">0.00 ms</b></span>
      </div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">NeonDB Serverless Pool Health</div>
          <div class="card-val" id="kpi-db-ping">0.00 ms</div>
        </div>
        <div class="kpi-icon water">🗄️</div>
      </div>
      <div class="card-sub">
        <span id="kpi-db-pool">Pool: 0 / 10</span>
        <span id="kpi-db-idle">Idle: 0</span>
      </div>
    </div>
    <div class="card">
      <div class="kpi-top">
        <div>
          <div class="card-label">Security &amp; HTTP Status Distribution</div>
          <div class="card-val" id="kpi-2xx">0 (2xx)</div>
        </div>
        <div class="kpi-icon green">🛡️</div>
      </div>
      <div class="card-sub">
        <span>4xx Guard: <b id="kpi-4xx">0</b></span>
        <span>5xx Err: <b id="kpi-5xx">0</b> (<span id="kpi-err-rate">0%</span>)</span>
      </div>
    </div>
  </section>

  <section class="card" style="margin-bottom:18px">
    <div class="section-head">
      <h2>🎯 Core Mobile &amp; Telemetry API Request Watchlist</h2>
      <div style="display:flex;gap:8px;align-items:center">
        <span class="badge espresso">Live Status Tracker</span>
        <button class="btn" onclick="switchDashTab('monitoring', true)">👥 Buka User Online</button>
      </div>
    </div>
    <div class="diag-grid" id="apm-watchlist-grid"></div>
  </section>

  <section class="split">
    <div class="card">
      <div class="section-head">
        <h2>⚡ Per-Endpoint API Response Time &amp; Percentile Telemetry (P50 / P95 / P99)</h2>
        <span id="uptime-label" style="font-size:12px;color:var(--text-muted);font-weight:600">Uptime: 0s</span>
      </div>
      <div style="display:flex;flex-wrap:wrap;gap:10px;justify-content:space-between;align-items:center;margin-bottom:14px">
        <input id="apm-search" class="inp" placeholder="Cari endpoint route (/monitoring, /admin/users, /toolkit, /chat)..." oninput="applyApmFilters()" style="min-width:250px;flex:1">
        <div class="filter-chips" id="apm-status-chips">
          <button class="chip-btn active" onclick="setApmStatusFilter('ALL', this)">Semua</button>
          <button class="chip-btn" onclick="setApmStatusFilter('2XX', this)">✅ 2xx OK</button>
          <button class="chip-btn" onclick="setApmStatusFilter('ERR', this)">⚠️ 4xx / 5xx</button>
          <button class="chip-btn" onclick="setApmStatusFilter('WS', this)">🔌 WS 101</button>
        </div>
        <select id="apm-method-filter" class="inp" onchange="applyApmFilters()">
          <option value="ALL">Semua Method</option>
          <option value="GET">GET</option>
          <option value="POST">POST</option>
          <option value="PUT">PUT</option>
          <option value="PATCH">PATCH</option>
          <option value="DELETE">DELETE</option>
        </select>
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

    <div style="display:flex;flex-direction:column;gap:18px">
      <div class="card">
        <div class="section-head">
          <h2>📱 Live NeonDB Domain Records</h2>
          <span class="badge ok">● Synced</span>
        </div>
        <div class="counts-grid" id="domain-counts"></div>
      </div>

      <div class="card">
        <div class="section-head">
          <h2>📡 Live Request Trace Stream</h2>
          <span class="badge espresso" id="trace-count-badge">Latest</span>
        </div>
        <div class="trace-list" id="trace-stream"></div>
      </div>
    </div>
  </section>
</div>
"#;
