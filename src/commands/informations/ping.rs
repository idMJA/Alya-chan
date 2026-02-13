use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use std::time::SystemTime;
use twilight_model::channel::message::component::{
    Component, Container, Separator, SeparatorSpacingSize, TextDisplay,
};
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;

pub struct PingCommand;

#[async_trait]
impl SlashCommand for PingCommand {
    fn name(&self) -> &'static str {
        "ping"
    }

    fn description(&self) -> &'static str {
        "Check bot latency and responsiveness"
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let interaction_created_ms = (ctx.interaction_id.get() >> 22) + DISCORD_EPOCH_MS;
        let client_ping = now_ms.saturating_sub(interaction_created_ms);

        let ping_status = match client_ping {
            0..=50 => "Excellent",
            51..=100 => "Great",
            101..=200 => "Good",
            201..=400 => "Fair",
            _ => "Poor",
        };

        let container = Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("## {} **Pong!**", ctx.bot.config.emoji.ping),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: Some(true),
                    spacing: None,
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "{} **Latency Status**\n`{}ms`",
                        ctx.bot.config.emoji.clock, client_ping
                    ),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("Status: **{}**", ping_status),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "{} **System Information**\nShards: `{}`",
                        ctx.bot.config.emoji.info, ctx.bot.shard_count
                    ),
                }),
            ],
            accent_color: Some(Some(ctx.bot.config.color.primary)),
            spoiler: None,
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
                        flags: Some(MessageFlags::IS_COMPONENTS_V2),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
