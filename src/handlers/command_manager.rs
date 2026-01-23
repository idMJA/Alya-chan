use crate::types::{BotContext, BotResult, CommandMeta, SlashCommand, SlashCommandContext};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

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

    pub fn register(&mut self, category: &str, command: Arc<dyn SlashCommand>) {
        let name = command.name().to_lowercase();
        let category_string = category.to_string();

        self.commands
            .insert(name.clone(), (command.clone(), category_string.clone()));

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

    pub fn get(&self, name: &str) -> Option<&Arc<dyn SlashCommand>> {
        self.commands.get(&name.to_lowercase()).map(|(cmd, _)| cmd)
    }

    pub fn get_all_commands(&self) -> Vec<&Arc<dyn SlashCommand>> {
        self.commands.values().map(|(cmd, _)| cmd).collect()
    }

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

    pub fn get_all_categories(&self) -> Vec<&str> {
        self.commands_by_category
            .keys()
            .map(|s| s.as_str())
            .collect()
    }

    #[allow(dead_code)]
    pub fn get_all_metadata(&self) -> Vec<CommandMeta> {
        self.commands
            .values()
            .map(|(cmd, _)| cmd.metadata())
            .collect()
    }

    #[allow(dead_code)]
    pub async fn execute(
        &self,
        bot: BotContext,
        name: &str,
        interaction_id: twilight_model::id::Id<twilight_model::id::marker::InteractionMarker>,
        application_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
        author_id: Option<twilight_model::id::Id<twilight_model::id::marker::UserMarker>>,
        guild_id: Option<twilight_model::id::Id<twilight_model::id::marker::GuildMarker>>,
        token: String,
        data: twilight_model::application::interaction::application_command::CommandData,
    ) -> BotResult<()> {
        if let Some(command) = self.get(name) {
            let ctx = SlashCommandContext::new(
                bot,
                interaction_id,
                application_id,
                author_id,
                guild_id,
                token,
                data,
            );
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
