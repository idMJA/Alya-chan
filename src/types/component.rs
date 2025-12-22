use super::{BotContext, BotResult};
use async_trait::async_trait;
use twilight_model::application::interaction::Interaction;

/// Context untuk component interactions (buttons, select menus, etc)
pub struct ComponentContext {
    #[allow(dead_code)]
    pub bot: BotContext,
    #[allow(dead_code)]
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

/// Trait untuk component handlers
#[async_trait]
pub trait ComponentHandler: Send + Sync {
    /// Custom ID pattern yang di-handle (contoh: "button_*", "select_role_*")
    fn custom_id_pattern(&self) -> &str;

    /// Handle component interaction
    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()>;
}
