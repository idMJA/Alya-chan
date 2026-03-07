use crate::database::service::AlyaDatabase;
use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{CommandBuilder, StringBuilder};

pub struct ChatbotLanguageCommand;

impl ChatbotLanguageCommand {
    fn normalize_language(input: &str) -> Option<(&'static str, &'static str)> {
        match input.trim().to_lowercase().as_str() {
            "id" | "indonesia" | "indonesian" | "bahasa" | "bahasa indonesia" => {
                Some(("id", "Indonesian"))
            }
            "en" | "english" | "inggris" | "bahasa inggris" => Some(("en", "English")),
            _ => None,
        }
    }

    async fn respond(&self, ctx: &SlashCommandContext, content: String) -> BotResult<()> {
        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        content: Some(content),
                        flags: Some(MessageFlags::EPHEMERAL),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}

#[async_trait]
impl SlashCommand for ChatbotLanguageCommand {
    fn name(&self) -> &'static str {
        "chatbot-language"
    }

    fn description(&self) -> &'static str {
        "Set your personal chatbot response language"
    }

    fn build(&self) -> Command {
        CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput)
            .option(StringBuilder::new("language", "Language code: id or en").required(true))
            .build()
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let Some(author_id) = ctx.author_id else {
            return self
                .respond(ctx, "Could not detect your user id.".to_string())
                .await;
        };

        let language_input = ctx.data.options.iter().find_map(|opt| {
            if opt.name == "language" {
                if let CommandOptionValue::String(value) = &opt.value {
                    return Some(value.clone());
                }
            }
            None
        });

        let Some(language_input) = language_input else {
            return self
                .respond(
                    ctx,
                    "Please provide a language. Supported: id, en".to_string(),
                )
                .await;
        };

        let Some((normalized, display)) = Self::normalize_language(&language_input) else {
            return self
                .respond(
                    ctx,
                    "Invalid language. Supported values: id, en".to_string(),
                )
                .await;
        };

        let db = match AlyaDatabase::get() {
            Ok(db) => db,
            Err(e) => {
                return self.respond(ctx, format!("Database not ready: {e}")).await;
            }
        };

        if let Err(e) = db
            .set_user_chatbot_language(&author_id.to_string(), normalized)
            .await
        {
            return self
                .respond(ctx, format!("Failed to save language preference: {e}"))
                .await;
        }

        self.respond(
            ctx,
            format!("Your chatbot language is set to {display} (`{normalized}`)."),
        )
        .await
    }
}
