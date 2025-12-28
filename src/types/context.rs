use crate::config::Config;
use std::sync::Arc;
use twilight_cache_inmemory::InMemoryCache;
use twilight_http::Client as HttpClient;
use twilight_standby::Standby;

/// Global bot context yang di-share ke semua handlers
#[derive(Clone)]
#[allow(dead_code)]
pub struct BotContext {
    pub http: Arc<HttpClient>,
    #[allow(dead_code)]
    pub cache: Arc<InMemoryCache>,
    #[allow(dead_code)]
    pub standby: Arc<Standby>,
    #[allow(dead_code)]
    pub config: Arc<Config>,
}

impl BotContext {
    pub fn new(
        http: Arc<HttpClient>,
        cache: Arc<InMemoryCache>,
        standby: Arc<Standby>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            http,
            cache,
            standby,
            config,
        }
    }
}
