use super::{BotContext, BotResult};
use async_trait::async_trait;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_model::id::Id;
use twilight_util::builder::command::CommandBuilder;

pub struct SlashCommandContext {
    pub bot: BotContext,
    pub interaction_id: twilight_model::id::Id<twilight_model::id::marker::InteractionMarker>,
    pub application_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
    pub author_id: Option<Id<UserMarker>>,
    pub guild_id: Option<Id<GuildMarker>>,
    pub token: String,
    pub data: Box<CommandData>,
}

impl SlashCommandContext {
    pub fn new(
        bot: BotContext,
        interaction_id: twilight_model::id::Id<twilight_model::id::marker::InteractionMarker>,
        application_id: twilight_model::id::Id<twilight_model::id::marker::ApplicationMarker>,
        author_id: Option<Id<UserMarker>>,
        guild_id: Option<Id<GuildMarker>>,
        token: String,
        data: CommandData,
    ) -> Self {
        Self {
            bot,
            interaction_id,
            application_id,
            author_id,
            guild_id,
            token,
            data: Box::new(data),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommandMeta {
    pub name: String,
    pub description: String,
    #[allow(dead_code)]
    pub category: String,
}

#[async_trait]
pub trait SlashCommand: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn build(&self) -> Command {
        CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput).build()
    }

    fn metadata(&self) -> CommandMeta {
        CommandMeta {
            name: self.name().to_string(),
            description: self.description().to_string(),
            category: String::new(),
        }
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()>;
}
