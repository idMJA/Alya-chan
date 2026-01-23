use crate::database::service::AlyaDatabase;
use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use serde_json::json;
use twilight_model::{
    channel::{
        permission_overwrite::{PermissionOverwrite, PermissionOverwriteType},
        ChannelType,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
};
use twilight_util::builder::embed::EmbedBuilder;

pub struct GlobalChatCommand;

#[async_trait]
impl SlashCommand for GlobalChatCommand {
    fn name(&self) -> &str {
        "globalchat"
    }

    fn description(&self) -> &str {
        "Setup global chat channel for cross-server interaction"
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        // Check if global chat is configured
        let gc_config = match &ctx.bot.config.global_chat {
            Some(gc) if gc.enabled => gc,
            _ => {
                return self
                    .respond_error(ctx, "Global chat is not enabled on this bot.")
                    .await;
            }
        };

        let guild_id = match ctx.guild_id {
            Some(id) => id,
            None => {
                return self
                    .respond_error(ctx, "This command can only be used in a server.")
                    .await;
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

        // Prepare headers for API calls
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        if let Some(key) = &gc_config.api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", key).parse().unwrap(),
            );
        }

        // Check if guild is already registered
        let client = reqwest::Client::new();
        let list_response = client
            .get(format!("{}/list", gc_config.api_url))
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| crate::types::error::BotError::Other(e.to_string()))?;

        let list_data: serde_json::Value = list_response
            .json()
            .await
            .map_err(|e| crate::types::error::BotError::Other(e.to_string()))?;

        if let Some(guilds) = list_data.get("guilds").and_then(|g| g.as_array()) {
            if let Some(existing) = guilds.iter().find(|g| {
                g.get("id")
                    .and_then(|id| id.as_str())
                    .map(|id| id == guild_id.to_string())
                    .unwrap_or(false)
            }) {
                let channel_id = existing
                    .get("globalChannelId")
                    .and_then(|c| c.as_str())
                    .unwrap_or("unknown");
                return self
                    .respond_error(
                        ctx,
                        &format!(
                            "This server already has global chat set up in <#{}>.",
                            channel_id
                        ),
                    )
                    .await;
            }
        }

        // Parse options to get channel if provided
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

        let (channel_id, created_new) = if let Some(channel_id) = channel_id_opt {
            // Use existing channel
            (channel_id, false)
        } else {
            // Create new channel
            let bot_user = ctx.bot.cache.current_user().expect("Bot user not in cache");
            let bot_id = bot_user.id;

            let permission_overwrites = vec![
                PermissionOverwrite {
                    id: bot_id.cast(),
                    kind: PermissionOverwriteType::Member,
                    allow: twilight_model::guild::Permissions::VIEW_CHANNEL
                        | twilight_model::guild::Permissions::SEND_MESSAGES
                        | twilight_model::guild::Permissions::EMBED_LINKS
                        | twilight_model::guild::Permissions::READ_MESSAGE_HISTORY
                        | twilight_model::guild::Permissions::MANAGE_MESSAGES,
                    deny: twilight_model::guild::Permissions::empty(),
                },
                PermissionOverwrite {
                    id: guild_id.cast(),
                    kind: PermissionOverwriteType::Role,
                    allow: twilight_model::guild::Permissions::VIEW_CHANNEL
                        | twilight_model::guild::Permissions::SEND_MESSAGES
                        | twilight_model::guild::Permissions::READ_MESSAGE_HISTORY,
                    deny: twilight_model::guild::Permissions::empty(),
                },
            ];

            let new_channel = ctx
                .bot
                .http
                .create_guild_channel(guild_id, "🌐・global-chat")
                .kind(ChannelType::GuildText)
                .topic("Alya Global Chat - Connect with users from other servers!")
                .permission_overwrites(&permission_overwrites)
                .await?
                .model()
                .await
                .map_err(|e| crate::types::error::BotError::Other(e.to_string()))?;

            // Send welcome message
            let welcome_embed = EmbedBuilder::new()
                .color(ctx.bot.config.color.primary)
                .title(&format!(
                    "{} Welcome to Global Chat!",
                    ctx.bot.config.emoji.globe
                ))
                .description(&format!(
                    "This channel is connected to a network of servers using Alya-chan.\n\
                        Messages sent here will be broadcasted to all connected servers.\n\n\
                        **Rules:**\n\
                        {} Be respectful to all users\n\
                        {} Follow Discord's Terms of Service\n\
                        {} No spam or advertising\n\
                        {} Have fun and make new friends!",
                    ctx.bot.config.emoji.info,
                    ctx.bot.config.emoji.info,
                    ctx.bot.config.emoji.warn,
                    ctx.bot.config.emoji.heart
                ))
                .build();

            ctx.bot
                .http
                .create_message(new_channel.id)
                .embeds(&[welcome_embed])
                .await?;

            (new_channel.id, true)
        };

        // Create webhook
        let webhook = ctx
            .bot
            .http
            .create_webhook(channel_id, "Alya Global Chat")
            .await?
            .model()
            .await
            .map_err(|e| crate::types::error::BotError::Other(e.to_string()))?;

        // Register with API
        let register_body = json!({
            "guildId": guild_id.to_string(),
            "globalChannelId": channel_id.to_string(),
            "webhookId": webhook.id.to_string(),
            "webhookToken": webhook.token,
        });

        let register_response = client
            .post(format!("{}/add", gc_config.api_url))
            .headers(headers)
            .json(&register_body)
            .send()
            .await
            .map_err(|e| crate::types::error::BotError::Other(e.to_string()))?;

        if !register_response.status().is_success() {
            return self
                .respond_error(ctx, "Failed to register with global chat API.")
                .await;
        }

        // Save to database
        let db = match AlyaDatabase::get() {
            Ok(db) => db,
            Err(e) => {
                return self
                    .respond_error(ctx, &format!("Database not ready: {}", e))
                    .await;
            }
        };

        if let Err(e) = db
            .create_global_chat_channel(
                &guild_id.to_string(),
                &channel_id.to_string(),
                Some(&webhook.id.to_string()),
                webhook.token.as_deref(),
            )
            .await
        {
            return self
                .respond_error(ctx, &format!("Failed to save global chat setup: {}", e))
                .await;
        }

        // Send success response
        let success_message = if created_new {
            format!(
                "{} Global chat has been successfully set up in <#{}>!\n\
                {} Messages sent in this channel will be broadcasted to all connected servers.",
                ctx.bot.config.emoji.yes, channel_id, ctx.bot.config.emoji.globe
            )
        } else {
            format!(
                "{} Global chat has been successfully set up using <#{}>!\n\
                {} Messages sent in this channel will be broadcasted to all connected servers.",
                ctx.bot.config.emoji.yes, channel_id, ctx.bot.config.emoji.globe
            )
        };

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
            "Global chat setup completed for guild {} in channel {}",
            guild_id,
            channel_id
        );

        Ok(())
    }
}

impl GlobalChatCommand {
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
