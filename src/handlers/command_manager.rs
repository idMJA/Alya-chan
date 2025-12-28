use crate::types::{BotContext, BotResult, CommandMeta, SlashCommand, SlashCommandContext};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

/// Manager untuk menangani semua slash commands
pub struct CommandManager {
    commands: HashMap<String, (Arc<dyn SlashCommand>, String)>,
    commands_by_category: BTreeMap<String, Vec<String>>,
}

impl CommandManager {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            commands_by_category: BTreeMap::new(),
        }
    }

    /// Register command baru dengan kategori
    pub fn register(&mut self, category: &str, command: Arc<dyn SlashCommand>) {
        let name = command.name().to_lowercase();
        let category_string = category.to_string();

        self.commands
            .insert(name.clone(), (command.clone(), category_string.clone()));

        // Track by category
        self.commands_by_category
            .entry(category_string.clone())
            .or_default()
            .push(name.clone());

        tracing::info!(
            "Registered slash command: {} (category: {})",
            name,
            category_string
        );
    }

    /// Get command by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn SlashCommand>> {
        self.commands.get(&name.to_lowercase()).map(|(cmd, _)| cmd)
    }

    /// Get all unique commands
    pub fn get_all_commands(&self) -> Vec<&Arc<dyn SlashCommand>> {
        self.commands.values().map(|(cmd, _)| cmd).collect()
    }

    /// Get commands by category
    pub fn get_commands_by_category(&self, category: &str) -> Vec<&Arc<dyn SlashCommand>> {
        self.commands_by_category
            .get(category)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.commands.get(name).map(|(cmd, _)| cmd))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all categories
    pub fn get_all_categories(&self) -> Vec<&str> {
        self.commands_by_category
            .keys()
            .map(|s| s.as_str())
            .collect()
    }

    /// Get metadata untuk semua commands
    #[allow(dead_code)]
    pub fn get_all_metadata(&self) -> Vec<CommandMeta> {
        self.commands
            .values()
            .map(|(cmd, _)| cmd.metadata())
            .collect()
    }
    /// Execute slash command
    #[allow(dead_code)]
    pub async fn execute(
        &self,
        bot: BotContext,
        name: &str,
        interaction_id: twilight_model::id::Id<twilight_model::id::marker::InteractionMarker>,
        application_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
        author_id: Option<twilight_model::id::Id<twilight_model::id::marker::UserMarker>>,
        token: String,
    ) -> BotResult<()> {
        if let Some(command) = self.get(name) {
            let ctx =
                SlashCommandContext::new(bot, interaction_id, application_id, author_id, token);
            command.execute(&ctx).await?;
        }

        Ok(())
    }
}

impl Default for CommandManager {
    fn default() -> Self {
        Self::new()
    }
}
