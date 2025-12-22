use crate::types::{BotResult, EventContext, EventHandler};
use async_trait::async_trait;
use twilight_model::gateway::event::Event;

/// Event handler untuk message create (untuk logging atau auto-moderation)
pub struct MessageCreateHandler;

#[async_trait]
impl EventHandler for MessageCreateHandler {
    fn name(&self) -> &str {
        "message_create"
    }

    async fn handle(&self, ctx: &EventContext) -> BotResult<()> {
        if let Event::MessageCreate(msg) = &ctx.event {
            // Skip bot messages
            if msg.author.bot {
                return Ok(());
            }

            // Log message (bisa dikembangkan untuk auto-moderation, spam detection, dll)
            tracing::debug!(
                "Message from {}#{} in channel {}: {}",
                msg.author.name,
                msg.author.discriminator,
                msg.channel_id,
                msg.content
            );
        }

        Ok(())
    }
}
