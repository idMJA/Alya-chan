use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

pub struct UserInfoCommand;

#[async_trait]
impl SlashCommand for UserInfoCommand {
    fn name(&self) -> &str {
        "userinfo"
    }

    fn description(&self) -> &str {
        "Get information about a user"
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let response = "User info command executed. Support for user options coming soon!";

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        content: Some(response.to_string()),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
