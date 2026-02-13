use crate::types::{BotResult, ComponentContext, ComponentHandler};
use async_trait::async_trait;
use std::num::NonZeroU64;
use twilight_model::application::interaction::InteractionData;
use twilight_model::channel::message::component::{
    Component, Container, Separator, SeparatorSpacingSize, TextDisplay,
};
use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};

#[allow(dead_code)]
pub struct ModerationConfirm;
#[allow(dead_code)]
pub struct ModerationCancel;

#[async_trait]
impl ComponentHandler for ModerationConfirm {
    fn custom_id_pattern(&self) -> &'static str {
        "mod_confirm:*"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &ctx.interaction;
        let custom_id = match &interaction.data {
            Some(InteractionData::MessageComponent(data)) => data.custom_id.clone(),
            _ => return Ok(()),
        };

        // Expected: mod_confirm:action:guild_id:target_id
        let mut parts = custom_id.split(':');
        parts.next();
        let action = parts.next().unwrap_or("");
        let guild_id_raw = parts.next().and_then(|s| s.parse::<u64>().ok());
        let target_id_raw = parts.next().and_then(|s| s.parse::<u64>().ok());

        if guild_id_raw.is_none() || target_id_raw.is_none() {
            return Ok(());
        }

        let guild_id_raw = guild_id_raw.unwrap();
        let target_id_raw = target_id_raw.unwrap();

        let guild_id = match NonZeroU64::new(guild_id_raw) {
            Some(n) => Id::<GuildMarker>::new(n.into()),
            None => return Ok(()),
        };

        let target_id = match NonZeroU64::new(target_id_raw) {
            Some(n) => Id::<UserMarker>::new(n.into()),
            None => return Ok(()),
        };

        let app_id = interaction.application_id;
        let token = interaction.token.clone();

        // perform action
        let result_message = match action {
            "kick" => {
                let _ = ctx.bot.http.remove_guild_member(guild_id, target_id).await;
                format!(
                    "{} Successfully kicked <@{}>",
                    ctx.bot.config.emoji.yes, target_id_raw
                )
            }
            "ban" => {
                let _ = ctx.bot.http.create_ban(guild_id, target_id).await;
                format!(
                    "{} Successfully banned <@{}>",
                    ctx.bot.config.emoji.yes, target_id_raw
                )
            }
            "timeout" => {
                // best-effort: try to set communication_disabled_until to 1 hour from now
                use chrono::Duration as ChronoDuration;
                use twilight_model::util::Timestamp;
                let until = chrono::Utc::now() + ChronoDuration::hours(1);
                let until_secs = until.timestamp();
                let ts = Timestamp::from_secs(until_secs)
                    .unwrap_or_else(|_| Timestamp::from_secs(0).unwrap());
                let _ = ctx
                    .bot
                    .http
                    .update_guild_member(guild_id, target_id)
                    .communication_disabled_until(Some(ts))
                    .await;
                format!(
                    "{} Successfully timed out <@{}>",
                    ctx.bot.config.emoji.yes, target_id_raw
                )
            }
            _ => format!("{} Unknown action", ctx.bot.config.emoji.no),
        };

        // Replace message with result
        let container = Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("## Moderation\n{result_message}"),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
            ],
            accent_color: None,
            spoiler: None,
        };

        let _ = ctx
            .bot
            .http
            .interaction(app_id.cast())
            .update_response(&token)
            .components(Some(&[Component::Container(container)]))
            .await;

        Ok(())
    }
}

#[async_trait]
impl ComponentHandler for ModerationCancel {
    fn custom_id_pattern(&self) -> &'static str {
        "mod_cancel:*"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &ctx.interaction;
        let token = interaction.token.clone();
        let app_id = interaction.application_id;

        let container = Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("{} Action cancelled", ctx.bot.config.emoji.no),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
            ],
            accent_color: None,
            spoiler: None,
        };

        let _ = ctx
            .bot
            .http
            .interaction(app_id.cast())
            .update_response(&token)
            .components(Some(&[Component::Container(container)]))
            .await;

        Ok(())
    }
}
