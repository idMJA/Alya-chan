use crate::database::service::AlyaDatabase;
use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use tokio::time::{sleep, Duration};
use twilight_model::{
    channel::message::component::{
        ActionRow, Button, ButtonStyle, Component, Container, Separator, TextDisplay,
    },
    channel::message::MessageFlags,
    channel::{
        permission_overwrite::{PermissionOverwrite, PermissionOverwriteType},
        ChannelType,
    },
    guild::Permissions,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
};

pub struct ChatbotCommand;

impl ChatbotCommand {
    fn build_status_container(
        color: u32,
        emoji: &crate::config::EmojiConfig,
        configured: bool,
        channel_id: Option<u64>,
    ) -> Container {
        let (status_emoji, status_text) = if configured {
            (emoji.yes.as_str(), "Configured")
        } else {
            (emoji.no.as_str(), "Not Configured")
        };

        let mut details = format!("{} Status: `{}`", status_emoji, status_text);

        if let Some(channel_id) = channel_id {
            details.push_str(&format!(
                "\n{} **Channel**: <#{}>",
                emoji.folder, channel_id
            ));
            details.push_str(&format!(
                "\n{} **Description**: This channel is configured to receive AI chatbot responses when mentioned.",
                emoji.info
            ));
        } else {
            details.push_str(&format!(
                "\n{} **What is Chatbot?**: Let Alya respond to mentions and questions in a dedicated channel.",
                emoji.info
            ));
            details.push_str(&format!(
                "\n{} **Next Step**: Click the button below to set up chatbot for your server.",
                emoji.arrow_right
            ));
        }

        Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("## {} Chatbot Setup", emoji.robot),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: Some(true),
                    spacing: None,
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: details,
                }),
            ],
            accent_color: Some(Some(color)),
            spoiler: None,
        }
    }

    fn build_success_container(
        color: u32,
        emoji: &crate::config::EmojiConfig,
        channel_id: u64,
    ) -> Container {
        Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("## Chatbot Setup Complete\n{}", emoji.yes),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: Some(true),
                    spacing: None,
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "{} **Channel**\n<#{}>\n\n{} **Functionality**\nI will respond to messages in this channel when:\n• You mention me directly\n• You use the keyword 'alya'\n• Someone asks me a question\n\n{} **Note**\nThis channel is configured and ready to use. You can manage this setup anytime using `/chatbot` command.",
                        emoji.folder, channel_id, emoji.info, emoji.pencil
                    ),
                }),
            ],
            accent_color: Some(Some(color)),
            spoiler: None,
        }
    }

    fn build_error_container(
        color: u32,
        emoji: &crate::config::EmojiConfig,
        message: &str,
    ) -> Container {
        Container {
            id: None,
            components: vec![Component::TextDisplay(TextDisplay {
                id: None,
                content: format!("## Error\n{} {}", emoji.no, message),
            })],
            accent_color: Some(Some(color)),
            spoiler: None,
        }
    }

    fn build_action_row(guild_id: u64, configured: bool, expired: bool) -> Component {
        let (custom_id, label, style) = if configured {
            (
                format!("setup_del_chatbot_confirm:{}", guild_id),
                "Delete Chatbot Setup".to_string(),
                if expired {
                    ButtonStyle::Secondary
                } else {
                    ButtonStyle::Danger
                },
            )
        } else {
            (
                format!("chatbot_create:{}", guild_id),
                "Create Chatbot".to_string(),
                if expired {
                    ButtonStyle::Secondary
                } else {
                    ButtonStyle::Primary
                },
            )
        };

        Component::ActionRow(ActionRow {
            id: None,
            components: vec![Component::Button(Button {
                custom_id: Some(custom_id),
                disabled: expired,
                label: Some(label),
                style,
                emoji: None,
                url: None,
                id: None,
                sku_id: None,
            })],
        })
    }
}

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
                let channel_id_num = existing.channel_id.parse::<u64>().unwrap_or(0);

                let status_container = Self::build_status_container(
                    ctx.bot.config.color.primary,
                    &ctx.bot.config.emoji,
                    true,
                    Some(channel_id_num),
                );

                let action_row = Self::build_action_row(guild_id.get(), true, false);

                ctx.bot
                    .http
                    .interaction(ctx.application_id.cast())
                    .create_response(
                        ctx.interaction_id.cast(),
                        &ctx.token,
                        &InteractionResponse {
                            kind: InteractionResponseType::ChannelMessageWithSource,
                            data: Some(InteractionResponseData {
                                components: Some(vec![
                                    Component::Container(status_container.clone()),
                                    action_row,
                                ]),
                                flags: Some(
                                    MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2,
                                ),
                                ..Default::default()
                            }),
                        },
                    )
                    .await?;

                let http = ctx.bot.http.clone();
                let token = ctx.token.clone();
                let application_id = ctx.application_id;
                tokio::spawn(async move {
                    sleep(Duration::from_secs(60)).await;

                    let expired_button = Self::build_action_row(guild_id.get(), true, true);
                    let components = vec![Component::Container(status_container), expired_button];
                    let _ = http
                        .interaction(application_id.cast())
                        .update_response(&token)
                        .components(Some(&components))
                        .await;
                });

                return Ok(());
            }
            Err(e) => {
                return self
                    .respond_error(ctx, &format!("Failed to check chatbot setup: {}", e))
                    .await;
            }
            _ => {}
        }

        if channel_id_opt.is_none() {
            let status_container = Self::build_status_container(
                ctx.bot.config.color.primary,
                &ctx.bot.config.emoji,
                false,
                None,
            );

            let action_row = Self::build_action_row(guild_id.get(), false, false);

            ctx.bot
                .http
                .interaction(ctx.application_id.cast())
                .create_response(
                    ctx.interaction_id.cast(),
                    &ctx.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::ChannelMessageWithSource,
                        data: Some(InteractionResponseData {
                            components: Some(vec![
                                Component::Container(status_container.clone()),
                                action_row,
                            ]),
                            flags: Some(MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2),
                            ..Default::default()
                        }),
                    },
                )
                .await?;

            let http = ctx.bot.http.clone();
            let token = ctx.token.clone();
            let application_id = ctx.application_id;
            tokio::spawn(async move {
                sleep(Duration::from_secs(60)).await;

                let expired_button = Self::build_action_row(guild_id.get(), false, true);
                let components = vec![Component::Container(status_container), expired_button];
                let _ = http
                    .interaction(application_id.cast())
                    .update_response(&token)
                    .components(Some(&components))
                    .await;
            });

            return Ok(());
        }

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

        let channel_id = if let Some(cid) = channel_id_opt {
            cid
        } else {
            let bot_id = match ctx.bot.cache.current_user() {
                Some(user) => user.id,
                None => ctx.application_id.cast(),
            };

            let everyone_role_id = guild_id;

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

        let success_container = Self::build_success_container(
            ctx.bot.config.color.primary,
            &ctx.bot.config.emoji,
            channel_id.get(),
        );

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_followup(&ctx.token)
            .components(&[Component::Container(success_container)])
            .flags(MessageFlags::IS_COMPONENTS_V2)
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
        let container =
            Self::build_error_container(ctx.bot.config.color.no, &ctx.bot.config.emoji, message);

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        components: Some(vec![Component::Container(container)]),
                        flags: Some(MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2),
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
        let container =
            Self::build_error_container(ctx.bot.config.color.no, &ctx.bot.config.emoji, message);

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_followup(&ctx.token)
            .components(&[Component::Container(container)])
            .flags(MessageFlags::IS_COMPONENTS_V2)
            .await?;

        Ok(())
    }
}
