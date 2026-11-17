pub mod admin_infra_tab_html;
pub mod apm_tab_html;
pub mod monitoring_tab_html;
pub mod nav_shell_html;
pub mod theme_base_css;
pub mod theme_components_css;

use crate::modules::telemetry::{
    admin_infra_script::ADMIN_INFRA_JS,
    apm_render_script::APM_RENDER_JS,
    dashboard_script::DASHBOARD_JS,
    monitoring_render_script::MONITORING_RENDER_JS,
    monitoring_script::MONITORING_JS,
    ui::{
        admin_infra_tab_html::{ADMIN_TAB_HTML, SERVICES_TAB_HTML},
        apm_tab_html::APM_TAB_HTML,
        monitoring_tab_html::{MONITORING_MODAL_HTML, MONITORING_TAB_HTML},
        nav_shell_html::NAV_SHELL_HTML,
        theme_base_css::THEME_BASE_CSS,
        theme_components_css::THEME_COMPONENTS_CSS,
    },
};

pub fn render_dashboard_html(default_tab: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="id">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Coffee Labs — Command, Telemetry &amp; User Monitoring Hub</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@500;700&family=Poppins:wght@400;500;600;700;800&display=swap" rel="stylesheet">
<style>{base_css}{comp_css}</style>
</head>
<body data-default-tab="{default_tab}">
  <div class="shell-wrap">
    {nav_shell}
    {apm_tab}
    {monitoring_tab}
    {admin_tab}
    {services_tab}
  </div>
  {dossier_modal}
  <script>{apm_render_js}</script>
  <script>{dash_js}</script>
  <script>{mon_render_js}</script>
  <script>{mon_js}</script>
  <script>{adm_js}</script>
</body>
</html>"#,
        base_css = THEME_BASE_CSS,
        comp_css = THEME_COMPONENTS_CSS,
        default_tab = default_tab,
        nav_shell = NAV_SHELL_HTML,
        apm_tab = APM_TAB_HTML,
        monitoring_tab = MONITORING_TAB_HTML,
        admin_tab = ADMIN_TAB_HTML,
        services_tab = SERVICES_TAB_HTML,
        dossier_modal = MONITORING_MODAL_HTML,
        apm_render_js = APM_RENDER_JS,
        dash_js = DASHBOARD_JS,
        mon_render_js = MONITORING_RENDER_JS,
        mon_js = MONITORING_JS,
        adm_js = ADMIN_INFRA_JS,
    )
}
