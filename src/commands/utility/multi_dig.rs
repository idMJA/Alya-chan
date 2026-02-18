use crate::types::{AutocompleteContext, BotResult, SlashCommand, SlashCommandContext};
use crate::utils::dig as u;
use async_trait::async_trait;
use twilight_model::application::command::{
    Command, CommandOptionChoice, CommandOptionChoiceValue, CommandType,
};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{BooleanBuilder, CommandBuilder, StringBuilder};

pub struct MultiDigCommand;

#[async_trait]
impl SlashCommand for MultiDigCommand {
    fn name(&self) -> &'static str {
        "multi-dig"
    }

    fn description(&self) -> &'static str {
        "Perform a DNS lookup with multiple record types"
    }

    fn build(&self) -> Command {
        CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput)
            .option(StringBuilder::new("domain", "The domain to lookup").required(true))
            .option(
                StringBuilder::new(
                    "types",
                    "Space-separated DNS record types to lookup, * for all types",
                )
                .autocomplete(true),
            )
            .option(BooleanBuilder::new(
                "short",
                "Display the results in short form",
            ))
            .option(BooleanBuilder::new("cdflag", "Disable DNSSEC checking"))
            .option(StringBuilder::new("provider", "DNS provider to use").autocomplete(true))
            .build()
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let mut domain = None;
        let mut raw_types = None;
        let mut short = false;
        let mut cdflag = false;
        let mut provider = u::PROVIDERS[0].name.to_string();

        for opt in &ctx.data.options {
            match opt.name.as_str() {
                "domain" => {
                    if let CommandOptionValue::String(s) = &opt.value {
                        domain = Some(s.clone());
                    }
                }
                "types" => {
                    if let CommandOptionValue::String(s) = &opt.value {
                        raw_types = Some(s.clone());
                    }
                }
                "short" => {
                    if let CommandOptionValue::Boolean(b) = opt.value {
                        short = b;
                    }
                }
                "cdflag" => {
                    if let CommandOptionValue::Boolean(b) = opt.value {
                        cdflag = b;
                    }
                }
                "provider" => {
                    if let CommandOptionValue::String(s) = &opt.value {
                        provider.clone_from(s);
                    }
                }
                _ => {}
            }
        }

        let domain = u::dom(domain.unwrap_or_default().as_str())
            .ok_or("A domain name could not be parsed from the given input.")?;
        let types = u::types(raw_types.as_deref());

        let flags = if types.len() > 5 {
            MessageFlags::EPHEMERAL
        } else {
            MessageFlags::empty()
        };

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        flags: Some(flags),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        for (idx, chunk) in types.chunks(10).enumerate() {
            let req = u::Req {
                domain: domain.clone(),
                types: chunk.to_vec(),
                short,
                cdflag,
                provider: provider.clone(),
            };

            let key = u::put(req).await;
            if let Some((embeds, components)) = u::run(&key).await {
                if idx == 0 {
                    ctx.bot
                        .http
                        .interaction(ctx.application_id.cast())
                        .update_response(&ctx.token)
                        .embeds(Some(&embeds))
                        .components(Some(&components))
                        .await?;
                } else {
                    ctx.bot
                        .http
                        .interaction(ctx.application_id.cast())
                        .create_followup(&ctx.token)
                        .embeds(&embeds)
                        .components(&components)
                        .flags(flags)
                        .await?;
                }
            }
        }

        Ok(())
    }

    async fn autocomplete(&self, ctx: &AutocompleteContext) -> BotResult<()> {
        let mut focused_name = "";
        let mut focused_value = "";

        for opt in &ctx.data.options {
            if let CommandOptionValue::Focused(value, _) = &opt.value {
                focused_name = &opt.name;
                focused_value = value;
                break;
            }
        }

        let focused_lower = focused_value.trim().to_lowercase();

        let choices = match focused_name {
            "types" => {
                let trimmed = focused_value.trim();
                let (prefix, current) = if trimmed.contains(' ') {
                    let mut parts = trimmed.split_whitespace().collect::<Vec<_>>();
                    let current = parts.pop().unwrap_or("");
                    let prefix = if parts.is_empty() {
                        String::new()
                    } else {
                        format!("{} ", parts.join(" "))
                    };
                    (prefix, current)
                } else {
                    (String::new(), trimmed)
                };

                let mut out = Vec::new();
                if current.is_empty() || current == "*" {
                    out.push(CommandOptionChoice {
                        name: "*".to_string(),
                        name_localizations: None,
                        value: CommandOptionChoiceValue::String(format!("{prefix}*")),
                    });
                }

                out.extend(
                    u::ALL_TYPES
                        .iter()
                        .filter(|t| t.starts_with(&current.to_uppercase()))
                        .take(25usize.saturating_sub(out.len()))
                        .map(|t| CommandOptionChoice {
                            name: t.to_string(),
                            name_localizations: None,
                            value: CommandOptionChoiceValue::String(format!("{prefix}{t}")),
                        }),
                );

                out
            }
            "provider" => u::PROVIDERS
                .iter()
                .filter(|p| p.name.to_lowercase().contains(&focused_lower))
                .take(25)
                .map(|p| CommandOptionChoice {
                    name: p.name.to_string(),
                    name_localizations: None,
                    value: CommandOptionChoiceValue::String(p.name.to_string()),
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
                    data: Some(InteractionResponseData {
                        choices: Some(choices),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
