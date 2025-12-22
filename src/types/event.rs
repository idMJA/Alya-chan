use super::{BotContext, BotResult};
use async_trait::async_trait;
use twilight_model::gateway::event::Event;

/// Context untuk event handling
pub struct EventContext {
    #[allow(dead_code)]
    pub bot: BotContext,
    pub event: Event,
}

impl EventContext {
    pub fn new(bot: BotContext, event: Event) -> Self {
        Self { bot, event }
    }
}

/// Trait untuk event handlers
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Nama event handler
    fn name(&self) -> &str;

    /// Handle event
    async fn handle(&self, ctx: &EventContext) -> BotResult<()>;
}
