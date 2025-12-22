use crate::types::{BotContext, BotResult, ComponentContext, ComponentHandler};
use std::sync::Arc;
use twilight_model::application::interaction::{Interaction, InteractionData};

/// Manager untuk menangani component interactions (buttons, select menus, modals, etc)
pub struct ComponentManager {
    handlers: Vec<Arc<dyn ComponentHandler>>,
}

impl ComponentManager {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Register component handler baru
    #[allow(dead_code)]
    pub fn register(&mut self, handler: Arc<dyn ComponentHandler>) {
        tracing::info!(
            "Registered component handler: {}",
            handler.custom_id_pattern()
        );
        self.handlers.push(handler);
    }

    /// Process interaction dan cari handler yang cocok
    pub async fn process_interaction(
        &self,
        bot: BotContext,
        interaction: Interaction,
    ) -> BotResult<()> {
        // Extract custom_id dari interaction data
        let custom_id = if let Some(data) = &interaction.data {
            match data {
                InteractionData::MessageComponent(comp_data) => Some(comp_data.custom_id.as_str()),
                _ => None,
            }
        } else {
            None
        };

        if let Some(custom_id) = custom_id {
            // Cari handler yang cocok dengan custom_id
            for handler in &self.handlers {
                let pattern = handler.custom_id_pattern();

                // Simple pattern matching (bisa dikembangkan lebih lanjut)
                let matches = if let Some(prefix) = pattern.strip_suffix('*') {
                    custom_id.starts_with(prefix)
                } else {
                    custom_id == pattern
                };

                if matches {
                    let ctx = ComponentContext::new(bot.clone(), interaction.clone());

                    match handler.handle(&ctx).await {
                        Ok(_) => {
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
