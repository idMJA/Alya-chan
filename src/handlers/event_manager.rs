use crate::types::{BotContext, BotResult, EventContext, EventHandler};
use std::sync::Arc;
use twilight_model::gateway::event::Event;

pub struct EventManager {
    handlers: Vec<Arc<dyn EventHandler>>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn register(&mut self, handler: Arc<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    pub fn log_summary(&self) {
        let handler_names: Vec<&str> = self.handlers.iter().map(|h| h.name()).collect();
        tracing::info!(
            "Registered {} event handlers: {}",
            self.handlers.len(),
            handler_names.join(", ")
        );
    }

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
