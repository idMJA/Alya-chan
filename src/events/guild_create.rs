use crate::types::{BotResult, EventContext, EventHandler};
use async_trait::async_trait;
use twilight_model::gateway::event::Event;

/// Event handler untuk guild create (ketika bot join server)
pub struct GuildCreateHandler;

#[async_trait]
impl EventHandler for GuildCreateHandler {
    fn name(&self) -> &str {
        "guild_create"
    }

    async fn handle(&self, ctx: &EventContext) -> BotResult<()> {
        if let Event::GuildCreate(guild) = &ctx.event {
            tracing::info!("Joined guild (ID: {})", guild.id());
        }

        Ok(())
    }
}
