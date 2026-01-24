use crate::database::service::AlyaDatabase;
use crate::types::{BotResult, ComponentContext, ComponentHandler};
use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;
use twilight_model::application::interaction::InteractionData;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::permission_overwrite::{PermissionOverwrite, PermissionOverwriteType};
use twilight_model::channel::ChannelType;
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::embed::EmbedBuilder;

pub struct GlobalChatCreateButton;

#[async_trait]
impl ComponentHandler for GlobalChatCreateButton {
    fn custom_id_pattern(&self) -> &str {
        "globalchat_create_*"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &ctx.interaction;
        let application_id = interaction.application_id;
        let interaction_id = interaction.id;
        let token = interaction.token.clone();

        let custom_id = match &interaction.data {
            Some(InteractionData::MessageComponent(data)) => data.custom_id.clone(),
            _ => return Ok(()),
        };

        // Expected format: globalchat_create:<guild_id>
        let mut parts = custom_id.split(':');
        parts.next(); // skip "globalchat_create"
        let guild_id = match parts.next().and_then(|id| id.parse::<u64>().ok()) {
            Some(id) => twilight_model::id::Id::<twilight_model::id::marker::GuildMarker>::new(id),
            None => {
                return self.respond_error(ctx, "Invalid guild ID").await;
            }
        };

        // Defer the response for long operation
        ctx.bot
            .http
            .interaction(application_id.cast())
            .create_response(
                interaction_id.cast(),
                &token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredUpdateMessage,
                    data: None,
                },
            )
            .await?;

        // Check if global chat is configured
        let gc_config = match &ctx.bot.config.global_chat {
            Some(gc) if gc.enabled => gc,
            _ => {
                return self
                    .respond_error_followup(ctx, "Global chat is not enabled on this bot.")
                    .await;
            }
        };

        // Create new channel
        let bot_id = match ctx.bot.cache.current_user() {
            Some(user) => user.id,
            None => application_id.cast(),
        };

        let permission_overwrites = vec![
            PermissionOverwrite {
                id: bot_id.cast(),
                kind: PermissionOverwriteType::Member,
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::EMBED_LINKS
                    | Permissions::READ_MESSAGE_HISTORY
                    | Permissions::MANAGE_MESSAGES,
                deny: Permissions::empty(),
            },
            PermissionOverwrite {
                id: guild_id.cast(),
                kind: PermissionOverwriteType::Role,
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
            },
        ];

        let new_channel = match ctx
            .bot
            .http
            .create_guild_channel(guild_id, "🌐・global-chat")
            .kind(ChannelType::GuildText)
            .topic("Alya Global Chat - Connect with users from other servers!")
            .permission_overwrites(&permission_overwrites)
            .await
        {
            Ok(resp) => match resp.model().await {
                Ok(ch) => ch,
                Err(e) => {
                    return self
                        .respond_error_followup(ctx, &format!("Failed to create channel: {}", e))
                        .await;
                }
            },
            Err(e) => {
                return self
                    .respond_error_followup(ctx, &format!("Failed to create channel: {}", e))
                    .await;
            }
        };

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

        if let Err(e) = ctx
            .bot
            .http
            .create_message(new_channel.id)
            .embeds(&[welcome_embed])
            .await
        {
            return self
                .respond_error_followup(ctx, &format!("Failed to send welcome message: {}", e))
                .await;
        }

        // Create webhook
        let webhook = match ctx
            .bot
            .http
            .create_webhook(new_channel.id, "Alya Global Chat")
            .await
        {
            Ok(resp) => match resp.model().await {
                Ok(wh) => wh,
                Err(e) => {
                    return self
                        .respond_error_followup(ctx, &format!("Failed to create webhook: {}", e))
                        .await;
                }
            },
            Err(e) => {
                return self
                    .respond_error_followup(ctx, &format!("Failed to create webhook: {}", e))
                    .await;
            }
        };

        // Prepare headers for API calls
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        if let Some(key) = &gc_config.api_key {
            headers.insert(AUTHORIZATION, format!("Bearer {}", key).parse().unwrap());
        }

        // Register with API
        let register_body = json!({
            "guildId": guild_id.to_string(),
            "globalChannelId": new_channel.id.to_string(),
            "webhookId": webhook.id.to_string(),
            "webhookToken": webhook.token,
        });

        let client = reqwest::Client::new();
        let register_response = match client
            .post(format!("{}/add", gc_config.api_url))
            .headers(headers)
            .json(&register_body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                return self
                    .respond_error_followup(ctx, &format!("Failed to register with API: {}", e))
                    .await;
            }
        };

        if !register_response.status().is_success() {
            return self
                .respond_error_followup(ctx, "Failed to register with global chat API.")
                .await;
        }

        // Save to database
        let db = match AlyaDatabase::get() {
            Ok(db) => db,
            Err(e) => {
                return self
                    .respond_error_followup(ctx, &format!("Database not ready: {}", e))
                    .await;
            }
        };

        if let Err(e) = db
            .create_global_chat_channel(
                &guild_id.to_string(),
                &new_channel.id.to_string(),
                Some(&webhook.id.to_string()),
                webhook.token.as_deref(),
            )
            .await
        {
            return self
                .respond_error_followup(ctx, &format!("Failed to save global chat setup: {}", e))
                .await;
        }

        // Send success response
        let success_message = format!(
            "{} Global chat has been successfully set up in <#{}>!\n\
            {} Messages sent in this channel will be broadcasted to all connected servers.",
            ctx.bot.config.emoji.yes, new_channel.id, ctx.bot.config.emoji.globe
        );

        let success_embed = EmbedBuilder::new()
            .color(ctx.bot.config.color.primary)
            .description(success_message)
            .build();

        ctx.bot
            .http
            .interaction(application_id.cast())
            .create_followup(&token)
            .embeds(&[success_embed])
            .await?;

        tracing::info!(
            "Global chat setup completed for guild {} in channel {}",
            guild_id,
            new_channel.id
        );

        Ok(())
    }
}

impl GlobalChatCreateButton {
    async fn respond_error(&self, ctx: &ComponentContext, message: &str) -> BotResult<()> {
        let embed = EmbedBuilder::new()
            .color(ctx.bot.config.color.no)
            .description(&format!("{} {}", ctx.bot.config.emoji.no, message))
            .build();

        ctx.bot
            .http
            .interaction(ctx.interaction.application_id.cast())
            .create_response(
                ctx.interaction.id.cast(),
                &ctx.interaction.token,
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

    async fn respond_error_followup(&self, ctx: &ComponentContext, message: &str) -> BotResult<()> {
        let embed = EmbedBuilder::new()
            .color(ctx.bot.config.color.no)
            .description(&format!("{} {}", ctx.bot.config.emoji.no, message))
            .build();

        ctx.bot
            .http
            .interaction(ctx.interaction.application_id.cast())
            .create_followup(&ctx.interaction.token)
            .embeds(&[embed])
            .await?;

        Ok(())
    }
}
