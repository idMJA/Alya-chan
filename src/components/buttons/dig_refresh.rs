use crate::types::{BotResult, ComponentContext, ComponentHandler};
use crate::utils::dig as u;
use async_trait::async_trait;
use twilight_model::application::interaction::InteractionData;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

pub struct DigRefreshButton;

#[async_trait]
impl ComponentHandler for DigRefreshButton {
    fn custom_id_pattern(&self) -> &'static str {
        "dig_refresh:*"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &ctx.interaction;
        let custom_id = match &interaction.data {
            Some(InteractionData::MessageComponent(data)) => data.custom_id.as_str(),
            _ => return Ok(()),
        };

        let key = custom_id.strip_prefix("dig_refresh:").unwrap_or_default();
        if let Some((embeds, components)) = u::run(key).await {
            ctx.bot
                .http
                .interaction(interaction.application_id.cast())
                .create_response(
                    interaction.id.cast(),
                    &interaction.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::UpdateMessage,
                        data: Some(InteractionResponseData {
                            embeds: Some(embeds),
                            components: Some(components),
                            ..Default::default()
                        }),
                    },
                )
                .await?;
        }

        Ok(())
    }
}
