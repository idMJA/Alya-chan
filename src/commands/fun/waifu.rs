use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::channel::message::component::{
    Component, Container, MediaGallery, MediaGalleryItem, Separator, SeparatorSpacingSize,
    TextDisplay, UnfurledMediaItem,
};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

pub struct WaifuCommand;

#[async_trait]
impl SlashCommand for WaifuCommand {
    fn name(&self) -> &str {
        "waifu"
    }

    fn description(&self) -> &str {
        "Get a random waifu image"
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let api_url = "https://api.waifu.pics/sfw/waifu";
        let client = reqwest::Client::new();

        let image_url = match client.get(api_url).send().await {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(json) => json
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                Err(_) => None,
            },
            Err(_) => None,
        };

        let container = if let Some(url) = image_url {
            Container {
                id: None,
                components: vec![
                    Component::TextDisplay(TextDisplay {
                        id: None,
                        content: "# Alya-chan".to_string(),
                    }),
                    Component::MediaGallery(MediaGallery {
                        id: None,
                        items: vec![MediaGalleryItem {
                            media: UnfurledMediaItem {
                                url: url.clone(),
                                content_type: None,
                                height: None,
                                width: None,
                                proxy_url: None,
                            },
                            description: Some("Here's your waifu!".to_string()),
                            spoiler: None,
                        }],
                    }),
                    Component::Separator(Separator {
                        id: None,
                        divider: None,
                        spacing: Some(SeparatorSpacingSize::Large),
                    }),
                    Component::TextDisplay(TextDisplay {
                        id: None,
                        content: "Source: waifu.pics".to_string(),
                    }),
                ],
                accent_color: Some(Some(ctx.bot.config.color.primary)),
                spoiler: None,
            }
        } else {
            Container {
                id: None,
                components: vec![
                    Component::TextDisplay(TextDisplay {
                        id: None,
                        content: "❌ **Error**\n\nOops! Something went wrong while fetching your waifu. Please try again later.".to_string(),
                    }),
                    Component::Separator(Separator {
                        id: None,
                        divider: None,
                        spacing: Some(SeparatorSpacingSize::Large),
                    }),
                    Component::TextDisplay(TextDisplay {
                        id: None,
                        content: "Source: waifu.pics".to_string(),
                    }),
                ],
                accent_color: Some(Some(ctx.bot.config.color.primary)),
                spoiler: None,
            }
        };

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
                        flags: Some(
                            twilight_model::channel::message::MessageFlags::IS_COMPONENTS_V2,
                        ),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
