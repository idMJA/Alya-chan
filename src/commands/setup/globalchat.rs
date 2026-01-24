use crate::database::service::AlyaDatabase;
use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use serde_json::json;
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

        // Parse response: data.guilds[...] with id and globalChannelId
        let existing_channel_id = list_data
            .get("data")
            .and_then(|data| data.get("guilds"))
            .and_then(|guilds| guilds.as_array())
            .and_then(|guilds| {
                guilds
                    .iter()
                    .find(|g| {
                        g.get("id")
                            .and_then(|id| id.as_str())
                            .map(|id| id == guild_id.to_string())
                            .unwrap_or(false)
                    })
                    .and_then(|g| {
                        g.get("globalChannelId")
                            .and_then(|c| c.as_str())
                            .map(String::from)
                    })
            });

        // Show status menu with embed (aesthetic design)
        let status_embed = if let Some(ref channel_id) = existing_channel_id {
            EmbedBuilder::new()
                .color(ctx.bot.config.color.primary)
                .title(&format!("{} Global Chat Status", ctx.bot.config.emoji.globe))
                .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                    &format!("{} Status", ctx.bot.config.emoji.yes),
                    "**Registered**",
                ))
                .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                    &format!("{} Channel", ctx.bot.config.emoji.folder),
                    &format!("<#{}>", channel_id),
                ))
                .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                    &format!("{} Description", ctx.bot.config.emoji.info),
                    "This server is connected to the global chat network. Messages sent in this channel will be broadcasted to all connected servers.",
                ))
                .build()
        } else {
            EmbedBuilder::new()
                .color(ctx.bot.config.color.primary)
                .title(&format!("{} Global Chat Status", ctx.bot.config.emoji.warn))
                .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                    &format!("{} Status", ctx.bot.config.emoji.no),
                    "**Not Registered**",
                ))
                .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                    &format!("{} What is Global Chat?", ctx.bot.config.emoji.info),
                    "Connect your server to a network of other servers. Share messages across communities and make new friends!",
                ))
                .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                    &format!("{} Next Step", ctx.bot.config.emoji.arrow_right),
                    "Click the button below to set up global chat for your server.",
                ))
                .build()
        };

        // Create action button
        let action_row = Component::ActionRow(ActionRow {
            id: None,
            components: if existing_channel_id.is_some() {
                // Show delete button if already registered
                vec![Component::Button(Button {
                    custom_id: Some(format!("setup_del_globalchat_confirm:{}", guild_id)),
                    disabled: false,
                    label: Some("Delete Global Chat".to_string()),
                    style: ButtonStyle::Danger,
                    emoji: None,
                    url: None,
                    id: None,
                    sku_id: None,
                })]
            } else {
                // Show create button if not registered
                vec![Component::Button(Button {
                    custom_id: Some(format!("globalchat_create:{}", guild_id)),
                    disabled: false,
                    label: Some("Create Global Chat".to_string()),
                    style: ButtonStyle::Primary,
                    emoji: None,
                    url: None,
                    id: None,
                    sku_id: None,
                })]
            },
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
            .description(&format!("{} {}", ctx.bot.config.emoji.no, message))
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
