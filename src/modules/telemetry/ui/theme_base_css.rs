pub const THEME_BASE_CSS: &str = r#"
  :root {
    --bg-cream: #F9F6F0;
    --bg-warm: #EFE8E0;
    --card: #FFFFFF;
    --card-soft: #FAF7F2;
    --border: #E8DFD8;
    --border-strong: #DCCFC2;
    --roast: #6B4423;
    --roast-dark: #4E2F16;
    --roast-soft: #F6EFEA;
    --text-dark: #332B25;
    --text-muted: #8F857D;
    --amber: #ECA400;
    --gold: #D4AF37;
    --water-bg: #E8F3FA;
    --water-fg: #1976D2;
    --hot-bg: #FFF0ED;
    --hot-fg: #E64A19;
    --green-bg: #E8F5E9;
    --green-fg: #2E7D32;
    --green-dot: #43A047;
    --red-bg: #FFEBEE;
    --red-fg: #D32F2F;
    --ws-bg: #F3E5F5;
    --ws-fg: #7B1FA2;
    --shadow-sm: 0 4px 14px rgba(107, 68, 35, 0.06);
    --shadow-md: 0 12px 28px rgba(107, 68, 35, 0.08);
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: 'Poppins', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    background: radial-gradient(circle at 8% 0%, #EFE8E0 0%, #F9F6F0 52%, #F6F1E9 100%);
    color: var(--text-dark);
    min-height: 100vh;
    padding: 20px 26px 40px;
  }
  .shell-wrap { max-width: 1460px; margin: 0 auto; }
  .header-card {
    background: var(--card);
    border: 1.5px solid var(--border);
    border-radius: 22px;
    padding: 18px 22px;
    box-shadow: var(--shadow-md);
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    margin-bottom: 14px;
  }
  .brand { display: flex; align-items: center; gap: 14px; }
  .logo {
    width: 50px;
    height: 50px;
    border-radius: 16px;
    background: linear-gradient(135deg, #6B4423 0%, #4E2F16 100%);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 24px;
    color: #FFFFFF;
    box-shadow: 0 8px 20px rgba(107, 68, 35, 0.25);
    border: 2px solid rgba(236, 164, 0, 0.45);
  }
  .title h1 { font-size: 18.5px; font-weight: 800; letter-spacing: -0.02em; color: var(--text-dark); }
  .title p { font-size: 12px; color: var(--text-muted); margin-top: 2px; font-weight: 500; }
  .actions { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .btn {
    background: var(--roast-soft);
    color: var(--roast);
    border: 1.5px solid var(--border);
    padding: 9px 14px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.18s ease;
    text-decoration: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: inherit;
  }
  .btn:hover { background: var(--bg-warm); border-color: var(--roast); transform: translateY(-1px); }
  .btn.primary {
    background: linear-gradient(135deg, #ECA400 0%, #D49000 100%);
    color: #FFFFFF;
    border: none;
    box-shadow: 0 6px 16px rgba(236, 164, 0, 0.28);
  }
  .btn.espresso {
    background: linear-gradient(135deg, #6B4423 0%, #4E2F16 100%);
    color: #FFFFFF;
    border: none;
    box-shadow: 0 6px 16px rgba(107, 68, 35, 0.22);
  }
  .nav-pill-bar {
    background: var(--card);
    border: 1.5px solid var(--border);
    border-radius: 18px;
    padding: 8px;
    box-shadow: var(--shadow-sm);
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  .nav-group { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .nav-btn {
    background: var(--card-soft);
    color: var(--text-muted);
    border: 1.5px solid transparent;
    padding: 10px 16px;
    border-radius: 13px;
    font-size: 12.5px;
    font-weight: 700;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    transition: all 0.18s ease;
    font-family: inherit;
  }
  .nav-btn:hover { background: var(--roast-soft); color: var(--roast); }
  .nav-btn.active {
    background: linear-gradient(135deg, #6B4423 0%, #4E2F16 100%);
    color: #FFFFFF;
    box-shadow: 0 6px 16px rgba(107, 68, 35, 0.24);
  }
  .nav-count {
    font-size: 10.5px;
    font-weight: 800;
    padding: 2px 8px;
    border-radius: 99px;
    background: rgba(107, 68, 35, 0.12);
    color: var(--roast);
  }
  .nav-btn.active .nav-count { background: rgba(255, 255, 255, 0.2); color: #FFFFFF; }
  .nav-count.live { background: var(--green-bg); color: var(--green-fg); }
  .nav-btn.active .nav-count.live { background: #43A047; color: #FFFFFF; }
  .grid-4 { display: grid; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); gap: 14px; margin-bottom: 18px; }
  .card {
    background: var(--card);
    border: 1.5px solid var(--border);
    border-radius: 20px;
    padding: 18px 20px;
    box-shadow: var(--shadow-sm);
  }
  .kpi-top { display: flex; justify-content: space-between; align-items: flex-start; gap: 10px; }
  .kpi-icon {
    width: 38px;
    height: 38px;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 18px;
    flex-shrink: 0;
  }
  .kpi-icon.espresso { background: var(--roast-soft); color: var(--roast); }
  .kpi-icon.amber { background: #FFF8E1; color: var(--amber); }
  .kpi-icon.water { background: var(--water-bg); color: var(--water-fg); }
  .kpi-icon.green { background: var(--green-bg); color: var(--green-fg); }
  .kpi-icon.hot { background: var(--hot-bg); color: var(--hot-fg); }
  .card-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); font-weight: 700; }
  .card-val { font-size: 26px; font-weight: 800; margin: 6px 0 6px; color: var(--text-dark); letter-spacing: -0.02em; }
  .card-sub { font-size: 11.5px; color: var(--text-muted); display: flex; justify-content: space-between; align-items: center; gap: 8px; }
  .card-sub b { color: var(--roast); }
  @media (max-width: 760px) { body { padding: 12px; } .header-card { padding: 14px; } }
"#;
