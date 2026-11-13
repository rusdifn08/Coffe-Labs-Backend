pub mod coalesce_scheduler_worker;
pub mod stream_consumer_worker;

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::modules::notifications::{
    events::OmnichannelNotifEvent,
    services::omnichannel_router::OmnichannelRouter,
};

pub fn spawn_notification_workers(
    redis: Option<fred::clients::RedisClient>,
    local_rx: mpsc::UnboundedReceiver<OmnichannelNotifEvent>,
    router: Arc<OmnichannelRouter>,
) {
    tokio::spawn(stream_consumer_worker::run_stream_consumer(
        redis.clone(),
        local_rx,
        Arc::clone(&router),
    ));

    if let Some(client) = redis {
        tokio::spawn(coalesce_scheduler_worker::run_coalesce_scheduler(client));
    }
}
