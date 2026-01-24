use crate::database::service::AlyaDatabase;
use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::{
    channel::message::component::{ActionRow, Button, ButtonStyle, Component},
    channel::message::MessageFlags,
    channel::{
        permission_overwrite::{PermissionOverwrite, PermissionOverwriteType},
        ChannelType,
    },
    guild::Permissions,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
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

        // Parse channel option (optional). If not provided, create a new channel.
        let channel_id_opt = ctx.data.options.iter().find_map(|opt| {
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
        });

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
                // Show status with delete option using embed
                let channel_id_num = existing.channel_id.parse::<u64>().unwrap_or(0);

                let status_embed = EmbedBuilder::new()
                    .color(ctx.bot.config.color.primary)
                    .title(&format!("{} Chatbot Setup Status", ctx.bot.config.emoji.robot))
                    .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                        &format!("{} Status", ctx.bot.config.emoji.yes),
                        "**Already Configured**",
                    ))
                    .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                        &format!("{} Channel", ctx.bot.config.emoji.folder),
                        &format!("<#{}>", channel_id_num),
                    ))
                    .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                        &format!("{} Description", ctx.bot.config.emoji.info),
                        "This channel is configured to receive AI chatbot responses when mentioned.",
                    ))
                    .build();

                let action_row = Component::ActionRow(ActionRow {
                    id: None,
                    components: vec![Component::Button(Button {
                        custom_id: Some(format!("setup_del_chatbot_confirm:{}", guild_id)),
                        disabled: false,
                        label: Some("Delete Chatbot Setup".to_string()),
                        style: ButtonStyle::Danger,
                        emoji: None,
                        url: None,
                        id: None,
                        sku_id: None,
                    })],
                });

                ctx.bot
                    .http
                    .interaction(ctx.application_id.cast())
                    .create_response(
                        ctx.interaction_id.cast(),
                        &ctx.token,
                        &InteractionResponse {
                            kind: InteractionResponseType::ChannelMessageWithSource,
                            data: Some(InteractionResponseData {
                                embeds: Some(vec![status_embed]),
                                components: Some(vec![action_row]),
                                flags: Some(MessageFlags::EPHEMERAL),
                                ..Default::default()
                            }),
                        },
                    )
                    .await?;

                return Ok(());
            }
            Err(e) => {
                return self
                    .respond_error(ctx, &format!("Failed to check chatbot setup: {}", e))
                    .await;
            }
            _ => {}
        }

        // Defer reply for creation flow
        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        flags: Some(MessageFlags::EPHEMERAL),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        // Resolve channel: use provided or create new one
        let channel_id = if let Some(cid) = channel_id_opt {
            cid
        } else {
            // Create new channel with permission overwrites
            let bot_id = match ctx.bot.cache.current_user() {
                Some(user) => user.id,
                None => ctx.application_id.cast(),
            };

            // Get @everyone role ID (same as guild ID)
            let everyone_role_id = guild_id;

            // Permission overwrites
            let permission_overwrites = vec![
                PermissionOverwrite {
                    id: bot_id.cast(),
                    kind: PermissionOverwriteType::Member,
                    allow: Permissions::VIEW_CHANNEL
                        | Permissions::SEND_MESSAGES
                        | Permissions::READ_MESSAGE_HISTORY
                        | Permissions::MANAGE_MESSAGES,
                    deny: Permissions::empty(),
                },
                PermissionOverwrite {
                    id: everyone_role_id.cast(),
                    kind: PermissionOverwriteType::Role,
                    allow: Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                },
            ];

            let new_channel = ctx
                .bot
                .http
                .create_guild_channel(guild_id, "🤖・chatbot")
                .kind(ChannelType::GuildText)
                .topic("Chatbot responses and interactions")
                .permission_overwrites(&permission_overwrites)
                .await?
                .model()
                .await
                .map_err(|e| crate::types::error::BotError::Other(e.to_string()))?;

            new_channel.id
        };

        if let Err(e) = db
            .create_chatbot_setup(&guild_id.to_string(), &channel_id.to_string())
            .await
        {
            return self
                .respond_error_followup(ctx, &format!("Failed to save chatbot setup: {}", e))
                .await;
        }

        // Send success response with aesthetic embed
        let success_embed = EmbedBuilder::new()
            .color(ctx.bot.config.color.primary)
            .title(&format!("{} Chatbot Setup Complete", ctx.bot.config.emoji.yes))
            .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                &format!("{} Channel", ctx.bot.config.emoji.folder),
                &format!("<#{}>", channel_id),
            ))
            .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                &format!("{} Functionality", ctx.bot.config.emoji.info),
                "I will respond to messages in this channel when:\n\
                • You mention me directly\n\
                • You use the keyword 'alya'\n\
                • Someone asks me a question",
            ))
            .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                &format!("{} Note", ctx.bot.config.emoji.pencil),
                "This channel is configured and ready to use. You can manage this setup anytime using `/chatbot` command.",
            ))
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
            .title(&format!("{} Error", ctx.bot.config.emoji.no))
            .description(message)
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
                        flags: Some(MessageFlags::EPHEMERAL),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }

    async fn respond_error_followup(
        &self,
        ctx: &SlashCommandContext,
        message: &str,
    ) -> BotResult<()> {
        let embed = EmbedBuilder::new()
            .color(ctx.bot.config.color.no)
            .title(&format!("{} Error", ctx.bot.config.emoji.no))
            .description(message)
            .build();

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_followup(&ctx.token)
            .embeds(&[embed])
            .await?;

        Ok(())
    }
}
