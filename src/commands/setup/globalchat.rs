use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use tokio::time::{sleep, Duration};
use twilight_model::{
    channel::message::component::{
        ActionRow, Button, ButtonStyle, Component, Container, Separator, SeparatorSpacingSize,
        TextDisplay,
    },
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
};

pub struct GlobalChatCommand;

impl GlobalChatCommand {
    fn build_action_row(guild_id: u64, registered: bool, expired: bool) -> Component {
        let (custom_id, label, style) = if registered {
            (
                format!("setup_del_globalchat_confirm:{guild_id}"),
                "Delete Global Chat".to_string(),
                if expired {
                    ButtonStyle::Secondary
                } else {
                    ButtonStyle::Danger
                },
            )
        } else {
            (
                format!("globalchat_create:{guild_id}"),
                "Create Global Chat".to_string(),
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

    fn build_status_container(
        color: u32,
        emoji: &crate::config::EmojiConfig,
        registered: bool,
        channel_id: Option<&str>,
    ) -> Container {
        let (status_emoji, status_text, header_emoji) = if registered {
            (emoji.yes.as_str(), "Registered", emoji.globe.as_str())
        } else {
            (emoji.no.as_str(), "Not Registered", emoji.warn.as_str())
        };

        let mut details = format!("{status_emoji} Status: `{status_text}`");

        if let Some(channel_id) = channel_id {
            use std::fmt::Write;
            let _ = write!(details, "\n{} **Channel**: <#{}>", emoji.folder, channel_id);
            let _ = write!(details, "\n{} **Description**: This server is connected to the global chat network. Messages sent in this channel will be broadcasted to all connected servers.", emoji.info);
        } else {
            use std::fmt::Write;
            let _ = write!(details, "\n{} **What is Global Chat?**: Connect your server to a network of other servers. Share messages across communities and make new friends!", emoji.info);
            let _ = write!(
                details,
                "\n{} **Next Step**: Click the button below to set up global chat for your server.",
                emoji.arrow_right
            );
        }

        Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("## {header_emoji} Global Chat"),
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
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
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
}

#[async_trait]
impl SlashCommand for GlobalChatCommand {
    fn name(&self) -> &'static str {
        "globalchat"
    }

    fn description(&self) -> &'static str {
        "Setup global chat channel for cross-server interaction"
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let gc_config = match &ctx.bot.config.global_chat {
            Some(gc) if gc.enabled => gc,
            _ => {
                return self
                    .respond_error(ctx, "Global chat is not enabled on this bot.")
                    .await;
            }
        };

        let Some(guild_id) = ctx.guild_id else {
            return self
                .respond_error(ctx, "This command can only be used in a server.")
                .await;
        };

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        if let Some(key) = &gc_config.api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {key}").parse().unwrap(),
            );
        }

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
                            .is_some_and(|id| id == guild_id.to_string())
                    })
                    .and_then(|g| {
                        g.get("globalChannelId")
                            .and_then(|c| c.as_str())
                            .map(String::from)
                    })
            });

        let status_container = Self::build_status_container(
            ctx.bot.config.color.primary,
            &ctx.bot.config.emoji,
            existing_channel_id.is_some(),
            existing_channel_id.as_deref(),
        );

        let action_row =
            Self::build_action_row(guild_id.get(), existing_channel_id.is_some(), false);

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

            let expired_button =
                Self::build_action_row(guild_id.get(), existing_channel_id.is_some(), true);
            let components = vec![Component::Container(status_container), expired_button];
            let _ = http
                .interaction(application_id.cast())
                .update_response(&token)
                .components(Some(&components))
                .await;
        });

        Ok(())
    }
}

impl GlobalChatCommand {
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
}
