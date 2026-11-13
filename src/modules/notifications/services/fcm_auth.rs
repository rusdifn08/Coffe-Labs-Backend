use crate::{
    core::{cache::AppCaches, fcm_oauth::FcmOAuthManager},
    modules::notifications::error::{NotifError, NotifResult},
};

/// Adapter tipis modul notifications ke `core::fcm_oauth::FcmOAuthManager` + `AppCaches`.
#[derive(Clone)]
pub struct FcmAuthManager {
    inner: FcmOAuthManager,
    caches: AppCaches,
}

impl FcmAuthManager {
    pub fn new(inner: FcmOAuthManager, caches: AppCaches) -> Self {
        Self { inner, caches }
    }

    #[allow(dead_code)]
    pub fn is_configured(&self) -> bool {
        self.inner.is_configured()
    }

    pub fn project_id(&self) -> &str {
        self.inner.resolved_project_id()
    }

    pub async fn invalidate_cache(&self) {
        self.inner.invalidate_cache(&self.caches).await;
    }

    pub async fn get_bearer_token(&self) -> NotifResult<String> {
        self.inner
            .get_valid_bearer_token(&self.caches)
            .await
            .map_err(|err| NotifError::RetryableDelivery {
                retry_after_secs: Some(2),
                reason: err.to_string(),
            })
    }
}
