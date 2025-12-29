use crate::types::{BotResult, EventContext, EventHandler};
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
                "Bot is ready! Logged in as {}#{}",
                ready.user.name,
                ready.user.discriminator
            );

            tracing::info!("Connected to {} guilds", ready.guilds.len());
        }

        Ok(())
    }
}
