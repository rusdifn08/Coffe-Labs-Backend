pub const THEME_COMPONENTS_CSS: &str = r#"
  .split { display: grid; grid-template-columns: 2.05fr 1fr; gap: 18px; margin-bottom: 18px; }
  .split-equal { display: grid; grid-template-columns: 1fr 1fr; gap: 18px; margin-bottom: 18px; }
  @media (max-width: 1120px) { .split, .split-equal { grid-template-columns: 1fr; } }
  .tab-pane.hidden { display: none; }
  .section-head { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 10px; margin-bottom: 14px; }
  .section-head h2 { font-size: 14.5px; font-weight: 800; color: var(--roast); display: flex; align-items: center; gap: 8px; }
  .badge { display: inline-flex; align-items: center; gap: 5px; padding: 4px 10px; border-radius: 99px; font-size: 11px; font-weight: 700; }
  .badge.ok { background: var(--green-bg); color: var(--green-fg); }
  .badge.warn { background: #FFF8E1; color: #B45309; }
  .badge.err { background: var(--red-bg); color: var(--red-fg); }
  .badge.info { background: var(--water-bg); color: var(--water-fg); }
  .badge.ws { background: var(--ws-bg); color: var(--ws-fg); }
  .badge.espresso { background: var(--roast-soft); color: var(--roast); }
  .badge-method { padding: 4px 8px; border-radius: 8px; font-size: 10.5px; font-weight: 800; font-family: 'JetBrains Mono', monospace; display: inline-block; }
  .GET { background: var(--water-bg); color: var(--water-fg); }
  .POST { background: var(--green-bg); color: var(--green-fg); }
  .PUT, .PATCH { background: #FFF8E1; color: #B45309; }
  .DELETE { background: var(--red-bg); color: var(--red-fg); }
  table { width: 100%; border-collapse: collapse; font-size: 12px; }
  th {
    text-align: left; color: var(--text-muted); font-weight: 700; padding: 10px 10px;
    background: var(--card-soft); border-bottom: 1.5px solid var(--border); font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.04em;
  }
  th:first-child { border-top-left-radius: 10px; }
  th:last-child { border-top-right-radius: 10px; }
  td { padding: 11px 10px; border-bottom: 1px solid #F1EAE3; font-family: 'JetBrains Mono', monospace; vertical-align: middle; }
  tr:hover td { background: #FCF9F4; }
  .route-tag {
    display: inline-block; margin-left: 6px; padding: 2px 7px; border-radius: 6px;
    background: var(--roast-soft); color: var(--roast); font-family: 'Poppins', sans-serif; font-size: 10px; font-weight: 700;
  }
  .bar-wrap { width: 88px; height: 7px; background: var(--bg-warm); border-radius: 99px; overflow: hidden; }
  .bar-fill { height: 100%; background: linear-gradient(90deg, #43A047, #ECA400, #E64A19); border-radius: 99px; transition: width 0.3s ease; }
  .counts-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-top: 8px; }
  .mini-stat {
    background: var(--card-soft); border: 1.5px solid var(--border); padding: 12px 10px;
    border-radius: 14px; text-align: center; transition: transform 0.15s ease;
  }
  .mini-stat:hover { transform: translateY(-2px); border-color: var(--roast); }
  .mini-stat b { display: block; font-size: 19px; color: var(--roast); font-weight: 800; }
  .mini-stat span { font-size: 10.5px; color: var(--text-muted); font-weight: 600; }
  .trace-list { max-height: 360px; overflow-y: auto; display: flex; flex-direction: column; gap: 8px; }
  .trace-item {
    display: flex; justify-content: space-between; align-items: center; gap: 8px;
    padding: 9px 12px; border-radius: 12px; background: var(--card-soft);
    border: 1px solid var(--border); font-family: 'JetBrains Mono', monospace; font-size: 11.5px;
  }
  .filter-chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .chip-btn {
    background: var(--card-soft); color: var(--text-muted); border: 1.5px solid var(--border);
    padding: 5px 11px; border-radius: 99px; font-size: 11px; font-weight: 700; cursor: pointer; font-family: inherit;
  }
  .chip-btn.active { background: var(--roast); color: #FFFFFF; border-color: var(--roast); }
  .inp {
    background: var(--card-soft); color: var(--text-dark); border: 1.5px solid var(--border);
    padding: 8px 12px; border-radius: 11px; font-size: 12px; outline: none; font-family: inherit;
  }
  .inp:focus { border-color: var(--roast); background: #FFFFFF; }
  .auth-bar {
    display: flex; flex-wrap: wrap; gap: 12px; align-items: center; justify-content: space-between;
    background: linear-gradient(135deg, #FFFFFF 0%, #FAF5ED 100%);
    border: 1.5px solid var(--border); border-radius: 18px; padding: 12px 18px; margin-bottom: 16px;
    box-shadow: var(--shadow-sm);
  }
  .online-strip {
    display: flex; gap: 10px; overflow-x: auto; padding: 4px 2px 8px; margin-bottom: 16px;
  }
  .online-chip {
    flex: 0 0 auto; display: flex; align-items: center; gap: 10px; background: var(--card);
    border: 1.5px solid var(--border); border-radius: 16px; padding: 10px 14px; cursor: pointer;
    box-shadow: var(--shadow-sm); transition: all 0.16s ease;
  }
  .online-chip:hover { border-color: var(--roast); transform: translateY(-2px); }
  .avatar-circle {
    width: 36px; height: 36px; border-radius: 50%; background: var(--roast-soft); color: var(--roast);
    font-weight: 800; font-size: 14px; display: flex; align-items: center; justify-content: center;
    position: relative; flex-shrink: 0; border: 1.5px solid var(--border);
  }
  .dot-badge {
    position: absolute; right: -1px; bottom: -1px; width: 11px; height: 11px;
    border-radius: 50%; border: 2px solid #FFFFFF;
  }
  .dot-badge.on { background: var(--green-dot); box-shadow: 0 0 0 3px rgba(67, 160, 71, 0.22); }
  .dot-badge.off { background: #B0A69D; }
  .pill { padding: 3px 9px; border-radius: 99px; font-size: 10.5px; font-weight: 700; display: inline-block; }
  .pill.LOW { background: var(--green-bg); color: var(--green-fg); }
  .pill.MEDIUM { background: #FFF8E1; color: #B45309; }
  .pill.HIGH, .pill.DORMANT { background: var(--red-bg); color: var(--red-fg); }
  .clickable-row { cursor: pointer; }
  .diag-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 10px; margin-bottom: 18px; }
  .diag-item {
    background: var(--card); border: 1.5px solid var(--border); border-radius: 16px; padding: 12px 14px;
    display: flex; justify-content: space-between; align-items: center; gap: 10px; box-shadow: var(--shadow-sm);
  }
  .modal-bg {
    position: fixed; inset: 0; background: rgba(51, 43, 37, 0.45); backdrop-filter: blur(6px);
    display: none; align-items: center; justify-content: center; padding: 20px; z-index: 99;
  }
  .modal-bg.open { display: flex; }
  .modal-card {
    background: var(--bg-cream); border: 1.5px solid var(--border); border-radius: 24px;
    max-width: 780px; width: 100%; max-height: 88vh; overflow-y: auto; padding: 22px 24px; box-shadow: var(--shadow-md);
  }
  .feat-row { display: flex; justify-content: space-between; align-items: center; gap: 10px; margin: 8px 0; font-size: 12px; }
  .ram-pill { font-size: 11px; font-weight: 700; color: var(--green-fg); background: var(--green-bg); padding: 5px 11px; border-radius: 99px; }
"#;
