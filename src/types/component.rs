use super::{BotContext, BotResult};
use async_trait::async_trait;
use twilight_model::application::interaction::Interaction;

pub struct ComponentContext {
    pub bot: BotContext,
    pub interaction: Box<Interaction>,
}

impl ComponentContext {
    pub fn new(bot: BotContext, interaction: Interaction) -> Self {
        Self {
            bot,
            interaction: Box::new(interaction),
        }
    }
}

#[async_trait]
pub trait ComponentHandler: Send + Sync {
    fn custom_id_pattern(&self) -> &str;

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()>;
}
