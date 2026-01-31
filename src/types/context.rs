use crate::config::Config;
use crate::database::service::AlyaDatabase;
use crate::handlers::CommandManager;
use std::sync::Arc;
use tokio::sync::broadcast;
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
    #[allow(dead_code)]
    pub database: &'static AlyaDatabase,
    #[allow(dead_code)]
    pub command_manager: Arc<CommandManager>,
    pub shard_count: u32,
    pub presence_tx: broadcast::Sender<PresenceUpdate>,
}

#[derive(Clone, Debug)]
pub struct PresenceUpdate {
    pub activity_name: String,
    pub status: twilight_model::gateway::presence::Status,
}

impl BotContext {
    pub fn new(
        http: Arc<HttpClient>,
        cache: Arc<InMemoryCache>,
        standby: Arc<Standby>,
        config: Arc<Config>,
        database: &'static AlyaDatabase,
        command_manager: Arc<CommandManager>,
        shard_count: u32,
        presence_tx: broadcast::Sender<PresenceUpdate>,
    ) -> Self {
        Self {
            http,
            cache,
            standby,
            config,
            database,
            command_manager,
            shard_count,
            presence_tx,
        }
    }
}
