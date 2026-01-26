use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{CommandBuilder, UserBuilder};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder};
use uuid::Uuid;

pub struct IqCommand;

#[async_trait]
impl SlashCommand for IqCommand {
    fn name(&self) -> &str {
        "iq"
    }

    fn description(&self) -> &str {
        "Generate a random IQ score for fun"
    }

    fn build(&self) -> Command {
        CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput)
            .option(UserBuilder::new(
                "user",
                "The user you want to check the IQ of",
            ))
            .build()
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let mut target_user = ctx.author_id;
        for opt in &ctx.data.options {
            if opt.name == "user" {
                if let CommandOptionValue::User(id) = opt.value {
                    target_user = Some(id);
                }
            }
        }

        let seed = Uuid::new_v4().as_u128();
        let iq = 2 + (seed % 299) as u32; // 2..=300

        let (emoji, line) = if iq >= 80 {
            ("🧠", format!("IQ high **{}**. You're a genius!", iq))
        } else if iq <= 50 {
            (
                "📚",
                format!("IQ low **{}**. Keep learning and growing!", iq),
            )
        } else {
            ("🧪", format!("IQ is **{}**.", iq))
        };

        let target_mention = target_user
            .map(|id| format!("<@{}>", id))
            .unwrap_or_else(|| "someone".to_string());

        let embed = EmbedBuilder::new()
            .color(ctx.bot.config.color.primary)
            .title(&format!("{} IQ Test", emoji))
            .field(EmbedFieldBuilder::new("User", target_mention))
            .field(EmbedFieldBuilder::new("Result", line))
            .build();

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        embeds: Some(vec![embed]),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
