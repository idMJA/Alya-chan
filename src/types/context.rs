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
pub struct BotContext {
    pub http: Arc<HttpClient>,
    pub cache: Arc<InMemoryCache>,
    #[allow(dead_code)]
    pub standby: Arc<Standby>,
    pub config: Arc<Config>,
    pub database: &'static AlyaDatabase,
    pub command_manager: Arc<CommandManager>,
    pub shard_count: u32,
    pub presence_tx: broadcast::Sender<PresenceUpdate>,
    pub bot_user: twilight_model::user::CurrentUser,
}

#[derive(Clone, Debug)]
pub struct PresenceUpdate {
    pub activity_name: String,
    pub status: twilight_model::gateway::presence::Status,
}

impl BotContext {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        http: Arc<HttpClient>,
        cache: Arc<InMemoryCache>,
        standby: Arc<Standby>,
        config: Arc<Config>,
        database: &'static AlyaDatabase,
        command_manager: Arc<CommandManager>,
        shard_count: u32,
        presence_tx: broadcast::Sender<PresenceUpdate>,
        bot_user: twilight_model::user::CurrentUser,
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
            bot_user,
        }
    }
}
