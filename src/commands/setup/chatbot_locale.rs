use crate::database::service::AlyaDatabase;
use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::message::MessageFlags;
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{CommandBuilder, StringBuilder};

pub struct ChatbotLocaleCommand;

impl ChatbotLocaleCommand {
    fn normalize_locale(input: &str) -> Option<(&'static str, &'static str)> {
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
impl SlashCommand for ChatbotLocaleCommand {
    fn name(&self) -> &'static str {
        "chatbot-locale"
    }

    fn description(&self) -> &'static str {
        "Set chatbot language for this server"
    }

    fn build(&self) -> Command {
        let mut command =
            CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput)
                .option(StringBuilder::new("language", "Language code: id or en").required(true))
                .build();
        command.default_member_permissions = Some(Permissions::MANAGE_GUILD);
        command
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let Some(guild_id) = ctx.guild_id else {
            return self
                .respond(
                    ctx,
                    "This command can only be used in a server.".to_string(),
                )
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
                    "Please provide a language. Supported values: id, en".to_string(),
                )
                .await;
        };

        let Some((normalized, display)) = Self::normalize_locale(&language_input) else {
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
            .set_chatbot_locale(&guild_id.to_string(), normalized)
            .await
        {
            return self
                .respond(ctx, format!("Failed to save chatbot locale: {e}"))
                .await;
        }

        self.respond(
            ctx,
            format!("Chatbot language for this server is set to {display} (`{normalized}`)."),
        )
        .await
    }
}
