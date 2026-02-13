use super::{BotContext, BotResult};
use async_trait::async_trait;
use twilight_model::gateway::event::Event;

pub struct EventContext {
    pub bot: BotContext,
    pub event: Event,
}

impl EventContext {
    pub const fn new(bot: BotContext, event: Event) -> Self {
        Self { bot, event }
    }
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    fn name(&self) -> &str;

    async fn handle(&self, ctx: &EventContext) -> BotResult<()>;
}
