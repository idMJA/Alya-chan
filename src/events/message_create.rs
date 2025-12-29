use crate::types::{BotResult, EventContext, EventHandler};
use async_trait::async_trait;
use twilight_model::gateway::event::Event;

pub struct MessageCreateHandler;

#[async_trait]
impl EventHandler for MessageCreateHandler {
    fn name(&self) -> &str {
        "message_create"
    }

    async fn handle(&self, ctx: &EventContext) -> BotResult<()> {
        if let Event::MessageCreate(msg) = &ctx.event {
            if msg.author.bot {
                return Ok(());
            }

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
