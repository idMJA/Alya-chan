use crate::types::{BotResult, EventContext, EventHandler};
use crate::utils::constants::BOT_VERSION;
use async_trait::async_trait;
use twilight_model::gateway::event::Event;

pub struct ReadyHandler;

#[async_trait]
impl EventHandler for ReadyHandler {
    fn name(&self) -> &str {
        "ready"
    }

    async fn handle(&self, ctx: &EventContext) -> BotResult<()> {
        if let Event::Ready(ready) = &ctx.event {
            tracing::info!(
                "Bot is ready! Logged in as {}#{} [v{}]",
                ready.user.name,
                ready.user.discriminator,
                BOT_VERSION
            );

            tracing::info!("Connected to {} guilds", ready.guilds.len());
        }

        Ok(())
    }
}
