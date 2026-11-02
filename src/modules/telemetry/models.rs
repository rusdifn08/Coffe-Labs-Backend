use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DatabasePoolMetrics {
    pub connected: bool,
    pub ping_latency_ms: f64,
    pub pool_size: u32,
    pub idle_connections: usize,
    pub max_connections: u32,
    pub engine: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DomainEntityCounts {
    pub users: i64,
    pub active_sessions: i64,
    pub social_posts: i64,
    pub social_comments: i64,
    pub synced_recipes: i64,
    pub brew_logs: i64,
    pub curated_beans: i64,
    pub marketplace_orders: i64,
    pub feedbacks: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ServiceModuleInfo {
    pub module: String,
    pub name: String,
    pub base_path: String,
    pub mobile_feature: String,
    pub security_tier: String,
    pub status: String,
}
