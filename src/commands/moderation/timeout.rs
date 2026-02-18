use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::application::command::Command;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{CommandBuilder, IntegerBuilder, StringBuilder, UserBuilder};

pub struct TimeoutCommand;

impl TimeoutCommand {
    fn build_confirmation_text(target: u64, seconds: Option<i64>, reason: Option<&str>) -> String {
        use std::fmt::Write;
        let mut s = format!("Are you sure you want to timeout <@{target}>?");
        if let Some(sec) = seconds {
            let _ = write!(s, "\nDuration: {sec} seconds");
        }
        if let Some(r) = reason {
            let _ = write!(s, "\nReason: {r}");
        }
        s
    }

    fn build_container(content: &str) -> twilight_model::channel::message::component::Container {
        use twilight_model::channel::message::component::{
            Component, Container, Separator, SeparatorSpacingSize, TextDisplay,
        };
        Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("## Timeout Confirmation\n{content}"),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
            ],
            accent_color: None,
            spoiler: None,
        }
    }
}

#[async_trait]
impl SlashCommand for TimeoutCommand {
    fn name(&self) -> &'static str {
        "timeout"
    }

    fn description(&self) -> &'static str {
        "Timeout a user (communication disabled)"
    }

    fn build(&self) -> Command {
        CommandBuilder::new(
            self.name(),
            self.description(),
            twilight_model::application::command::CommandType::ChatInput,
        )
        .option(UserBuilder::new("user", "The user to timeout").required(true))
        .option(IntegerBuilder::new("seconds", "Duration in seconds").required(true))
        .option(StringBuilder::new("reason", "Reason for the timeout"))
        .build()
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let Some(guild_id) = ctx.guild_id else {
            return self
                .respond_error(ctx, "This command can only be used in a server")
                .await;
        };

        let mut target: Option<twilight_model::id::Id<twilight_model::id::marker::UserMarker>> =
            None;
        let mut seconds: Option<i64> = None;
        let mut reason: Option<String> = None;

        for opt in &ctx.data.options {
            if opt.name == "user" {
                if let CommandOptionValue::User(id) = opt.value {
                    target = Some(id);
                }
            }
            if opt.name == "seconds" {
                if let twilight_model::application::interaction::application_command::CommandOptionValue::Integer(i) = opt.value { seconds = Some(i); }
            }
            if opt.name == "reason" {
                if let twilight_model::application::interaction::application_command::CommandOptionValue::String(s) = &opt.value { reason = Some(s.clone()); }
            }
        }

        let Some(target) = target else {
            return self
                .respond_error(ctx, "You must specify a user to timeout")
                .await;
        };
        let Some(seconds) = seconds else {
            return self
                .respond_error(ctx, "You must specify a duration in seconds")
                .await;
        };

        let confirmation_text =
            Self::build_confirmation_text(target.get(), Some(seconds), reason.as_deref());
        let container = Self::build_container(&confirmation_text);

        let confirm_id = format!("mod_confirm:timeout:{}:{}", guild_id.get(), target.get());
        let cancel_id = format!("mod_cancel:{}", ctx.interaction_id);

        let confirm_emoji = crate::utils::emoji::parse_component_emoji(&ctx.bot.config.emoji.yes);
        let cancel_emoji = crate::utils::emoji::parse_component_emoji(&ctx.bot.config.emoji.no);

        let action_row = Component::ActionRow(ActionRow {
            id: None,
            components: vec![
                Component::Button(Button {
                    custom_id: Some(confirm_id),
                    disabled: false,
                    label: Some("Confirm".to_string()),
                    style: ButtonStyle::Danger,
                    emoji: confirm_emoji,
                    url: None,
                    id: None,
                    sku_id: None,
                }),
                Component::Button(Button {
                    custom_id: Some(cancel_id),
                    disabled: false,
                    label: Some("Cancel".to_string()),
                    style: ButtonStyle::Secondary,
                    emoji: cancel_emoji,
                    url: None,
                    id: None,
                    sku_id: None,
                }),
            ],
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
                        components: Some(vec![Component::Container(container), action_row]),
                        flags: Some(
                            twilight_model::channel::message::MessageFlags::EPHEMERAL
                                | twilight_model::channel::message::MessageFlags::IS_COMPONENTS_V2,
                        ),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}

impl TimeoutCommand {
    async fn respond_error(&self, ctx: &SlashCommandContext, message: &str) -> BotResult<()> {
        use twilight_model::channel::message::component::{Component, Container, TextDisplay};
        use twilight_model::channel::message::MessageFlags;

        let container = Container {
            id: None,
            components: vec![Component::TextDisplay(TextDisplay {
                id: None,
                content: format!("## Error\n{} {}", ctx.bot.config.emoji.no, message),
            })],
            accent_color: None,
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
                        flags: Some(MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
