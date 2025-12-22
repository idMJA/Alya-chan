use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

/// Slash command ping untuk test bot responsiveness
pub struct PingCommand;

#[async_trait]
impl SlashCommand for PingCommand {
    fn name(&self) -> &str {
        "ping"
    }

    fn description(&self) -> &str {
        "Check bot latency and responsiveness"
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        ctx.bot
            .http
            .interaction(ctx.interaction_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        content: Some("🏓 Pong!".to_string()),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
