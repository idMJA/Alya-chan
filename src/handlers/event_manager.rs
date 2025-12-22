use crate::types::{BotContext, BotResult, EventContext, EventHandler};
use std::sync::Arc;
use twilight_model::gateway::event::Event;

/// Manager untuk menangani semua events
pub struct EventManager {
    handlers: Vec<Arc<dyn EventHandler>>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Register event handler baru
    pub fn register(&mut self, handler: Arc<dyn EventHandler>) {
        tracing::info!("Registered event handler: {}", handler.name());
        self.handlers.push(handler);
    }

    /// Process event dengan semua registered handlers
    pub async fn process_event(&self, bot: BotContext, event: Event) -> BotResult<()> {
        let ctx = EventContext::new(bot, event);

        for handler in &self.handlers {
            if let Err(e) = handler.handle(&ctx).await {
                tracing::error!("Event handler '{}' failed: {}", handler.name(), e);
            }
        }

        Ok(())
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}
