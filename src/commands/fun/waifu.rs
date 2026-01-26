use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder, ImageSource};

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

        let mut embed = EmbedBuilder::new()
            .color(ctx.bot.config.color.primary)
            .title("Here's your waifu!");

        if let Some(url) = image_url {
            if let Ok(source) = ImageSource::url(url) {
                embed = embed.image(source);
            } else {
                embed = embed.description("Failed to fetch waifu image. Please try again later.");
            }
        } else {
            embed = embed.description("Failed to fetch waifu image. Please try again later.");
        }

        let embed = embed
            .footer(EmbedFooterBuilder::new("Source: waifu.pics"))
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
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
