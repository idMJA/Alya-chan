use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use std::time::SystemTime;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::embed::EmbedBuilder;

const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;

pub struct PingCommand;

#[async_trait]
impl SlashCommand for PingCommand {
    fn name(&self) -> &str {
        "ping"
    }

    fn description(&self) -> &str {
        "Check bot latency and responsiveness"
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let interaction_created_ms = (ctx.interaction_id.get() >> 22) + DISCORD_EPOCH_MS;
        let client_ping = now_ms.saturating_sub(interaction_created_ms);

        let ping_embed = EmbedBuilder::new()
            .color(ctx.bot.config.color.primary)
            .title(&format!("{} Pong!", ctx.bot.config.emoji.ping))
            .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                &format!("{} Client Ping", ctx.bot.config.emoji.clock),
                &format!("`{}ms`", client_ping),
            ))
            .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                &format!("{} Shard Count", ctx.bot.config.emoji.info),
                &format!("`{}`", ctx.bot.shard_count),
            ))
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
                        embeds: Some(vec![ping_embed]),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
