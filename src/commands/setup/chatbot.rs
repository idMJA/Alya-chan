use crate::database::service::AlyaDatabase;
use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::embed::EmbedBuilder;

pub struct ChatbotCommand;

#[async_trait]
impl SlashCommand for ChatbotCommand {
    fn name(&self) -> &str {
        "chatbot"
    }

    fn description(&self) -> &str {
        "Setup chatbot channel"
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let guild_id = match ctx.guild_id {
            Some(id) => id,
            None => {
                return self
                    .respond_error(ctx, "This command can only be used in a server.")
                    .await;
            }
        };

        // Parse channel option (required)
        let channel_id = match ctx.data.options.iter().find_map(|opt| {
            if opt.name == "channel" {
                match &opt.value {
                    twilight_model::application::interaction::application_command::CommandOptionValue::Channel(id) => {
                        Some(*id)
                    }
                    _ => None,
                }
            } else {
                None
            }
        }) {
            Some(id) => id,
            None => {
                return self.respond_error(ctx, "Please provide a valid channel.").await;
            }
        };

        // Defer reply
        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        flags: Some(twilight_model::channel::message::MessageFlags::EPHEMERAL),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        let db = match AlyaDatabase::get() {
            Ok(db) => db,
            Err(e) => {
                return self
                    .respond_error(ctx, &format!("Database not ready: {}", e))
                    .await;
            }
        };

        match db.get_chatbot_setup(&guild_id.to_string()).await {
            Ok(Some(existing)) => {
                return self
                    .respond_error(
                        ctx,
                        &format!(
                            "Chatbot channel is already set to <#{}>.\\nUse `/chatbot delete` to reset.",
                            existing.channel_id
                        ),
                    )
                    .await;
            }
            Err(e) => {
                return self
                    .respond_error(ctx, &format!("Failed to check chatbot setup: {}", e))
                    .await;
            }
            _ => {}
        }

        if let Err(e) = db
            .create_chatbot_setup(&guild_id.to_string(), &channel_id.to_string())
            .await
        {
            return self
                .respond_error(ctx, &format!("Failed to save chatbot setup: {}", e))
                .await;
        }

        // Send success response
        let success_message = format!(
            "{} Chatbot has been successfully set up in <#{}>!\n\
            I will respond to messages in this channel when mentioned or when 'alya' is mentioned.",
            ctx.bot.config.emoji.yes, channel_id
        );

        let success_embed = EmbedBuilder::new()
            .color(ctx.bot.config.color.primary)
            .description(success_message)
            .build();

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_followup(&ctx.token)
            .embeds(&[success_embed])
            .await?;

        tracing::info!(
            "Chatbot setup completed for guild {} in channel {}",
            guild_id,
            channel_id
        );

        Ok(())
    }
}

impl ChatbotCommand {
    async fn respond_error(&self, ctx: &SlashCommandContext, message: &str) -> BotResult<()> {
        let embed = EmbedBuilder::new()
            .color(ctx.bot.config.color.no)
            .description(&format!("{} {}", ctx.bot.config.emoji.no, message))
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
                        flags: Some(twilight_model::channel::message::MessageFlags::EPHEMERAL),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
