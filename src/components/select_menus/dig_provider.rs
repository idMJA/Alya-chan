use crate::types::{BotResult, ComponentContext, ComponentHandler};
use crate::utils::dig as u;
use async_trait::async_trait;
use twilight_model::application::interaction::InteractionData;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

pub struct DigProviderSelect;

#[async_trait]
impl ComponentHandler for DigProviderSelect {
    fn custom_id_pattern(&self) -> &'static str {
        "dig_provider:*"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &ctx.interaction;
        let (custom_id, value) = match &interaction.data {
            Some(InteractionData::MessageComponent(data)) => {
                (data.custom_id.as_str(), data.values.first().cloned())
            }
            _ => return Ok(()),
        };

        let key = custom_id.strip_prefix("dig_provider:").unwrap_or_default();
        if let Some(v) = value {
            u::set_provider(key, &v).await;
        }

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
