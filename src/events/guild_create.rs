use crate::types::{BotResult, EventContext, EventHandler};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use twilight_model::gateway::event::Event;

static BOT_STARTUP_COMPLETE: AtomicBool = AtomicBool::new(false);
static SEEN_GUILD_IDS: OnceLock<std::sync::RwLock<HashSet<u64>>> = OnceLock::new();

pub struct GuildCreateHandler;

impl GuildCreateHandler {
    fn get_seen_guilds() -> &'static std::sync::RwLock<HashSet<u64>> {
        SEEN_GUILD_IDS.get_or_init(|| std::sync::RwLock::new(HashSet::new()))
    }

    pub fn track_guild(guild_id: u64) {
        if let Ok(mut seen) = Self::get_seen_guilds().write() {
            seen.insert(guild_id);
        }
    }

    pub fn is_new_guild(guild_id: u64) -> bool {
        Self::get_seen_guilds()
            .read()
            .map_or(true, |seen| !seen.contains(&guild_id))
    }

    pub async fn startup_complete() {
        tokio::time::sleep(Duration::from_secs(5)).await;
        BOT_STARTUP_COMPLETE.store(true, Ordering::SeqCst);
    }

    pub fn is_startup_complete() -> bool {
        BOT_STARTUP_COMPLETE.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl EventHandler for GuildCreateHandler {
    fn name(&self) -> &'static str {
        "guild_create"
    }

    async fn handle(&self, ctx: &EventContext) -> BotResult<()> {
        if let Event::GuildCreate(guild) = &ctx.event {
            let guild_id = guild.id();

            // Track this guild as seen
            Self::track_guild(guild_id.get());

            // Only log if after startup AND this is a new guild
            if Self::is_startup_complete() && Self::is_new_guild(guild_id.get()) {
                tracing::info!("Joined guild (ID: {})", guild_id);
            }
        }

        Ok(())
    }
}
