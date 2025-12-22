use super::{BotContext, BotResult};
use async_trait::async_trait;

pub struct SlashCommandContext {
    pub bot: BotContext,
    pub interaction_id: twilight_model::id::Id<twilight_model::id::marker::InteractionMarker>,
    pub token: String,
}

impl SlashCommandContext {
    pub fn new(
        bot: BotContext,
        interaction_id: twilight_model::id::Id<twilight_model::id::marker::InteractionMarker>,
        token: String,
    ) -> Self {
        Self {
            bot,
            interaction_id,
            token,
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
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

    fn metadata(&self) -> CommandMeta {
        CommandMeta {
            name: self.name().to_string(),
            description: self.description().to_string(),
            category: String::new(),
        }
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()>;
}
