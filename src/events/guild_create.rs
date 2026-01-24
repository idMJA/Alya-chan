use crate::types::{BotResult, EventContext, EventHandler};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use twilight_model::gateway::event::Event;

static BOT_STARTUP_COMPLETE: AtomicBool = AtomicBool::new(false);

pub struct GuildCreateHandler;

impl GuildCreateHandler {
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
    fn name(&self) -> &str {
        "guild_create"
    }

    async fn handle(&self, ctx: &EventContext) -> BotResult<()> {
        if let Event::GuildCreate(guild) = &ctx.event {
            if Self::is_startup_complete() {
                tracing::info!("Joined guild (ID: {})", guild.id());
            }
        }

        Ok(())
    }
}
