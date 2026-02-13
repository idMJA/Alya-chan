use crate::types::{BotContext, BotResult, ComponentContext, ComponentHandler};
use std::sync::Arc;
use twilight_model::application::interaction::{Interaction, InteractionData};

pub struct ComponentManager {
    handlers: Vec<Arc<dyn ComponentHandler>>,
}

impl ComponentManager {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn register(&mut self, handler: Arc<dyn ComponentHandler>) {
        self.handlers.push(handler);
    }

    pub fn log_summary(&self) {
        let patterns: Vec<&str> = self
            .handlers
            .iter()
            .map(|h| h.custom_id_pattern())
            .collect();
        tracing::info!(
            "Registered {} component handlers: {}",
            self.handlers.len(),
            patterns.join(", ")
        );
    }

    pub async fn process_interaction(
        &self,
        bot: BotContext,
        interaction: Interaction,
    ) -> BotResult<()> {
        let custom_id = interaction.data.as_ref().and_then(|data| match data {
            InteractionData::MessageComponent(comp_data) => Some(comp_data.custom_id.as_str()),
            _ => None,
        });

        if let Some(custom_id) = custom_id {
            for handler in &self.handlers {
                let pattern = handler.custom_id_pattern();

                let matches = pattern.strip_suffix('*').map_or_else(
                    || custom_id == pattern,
                    |prefix| custom_id.starts_with(prefix),
                );

                if matches {
                    let ctx = ComponentContext::new(bot.clone(), interaction.clone());

                    match handler.handle(&ctx).await {
                        Ok(()) => {
                            tracing::info!("Component '{}' handled successfully", custom_id);
                            return Ok(());
                        }
                        Err(e) => {
                            tracing::error!("Component handler '{}' failed: {}", pattern, e);
                            return Err(e);
                        }
                    }
                }
            }

            tracing::warn!("No handler found for component: {}", custom_id);
        }

        Ok(())
    }
}

impl Default for ComponentManager {
    fn default() -> Self {
        Self::new()
    }
}
