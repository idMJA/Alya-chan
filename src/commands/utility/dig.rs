use crate::types::{AutocompleteContext, BotResult, SlashCommand, SlashCommandContext};
use crate::utils::dig as u;
use async_trait::async_trait;
use twilight_model::application::command::{
    Command, CommandOptionChoice, CommandOptionChoiceValue, CommandType,
};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{BooleanBuilder, CommandBuilder, StringBuilder};

pub struct DigCommand;

#[async_trait]
impl SlashCommand for DigCommand {
    fn name(&self) -> &'static str {
        "dig"
    }

    fn description(&self) -> &'static str {
        "Perform a DNS over Discord lookup"
    }

    fn build(&self) -> Command {
        CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput)
            .option(StringBuilder::new("domain", "The domain to lookup").required(true))
            .option(StringBuilder::new("type", "DNS record type to lookup").autocomplete(true))
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
        let mut t = "A".to_string();
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
                "type" => {
                    if let CommandOptionValue::String(s) = &opt.value {
                        t = s.to_uppercase();
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
        let req = u::Req {
            domain,
            types: vec![t],
            short,
            cdflag,
            provider,
        };

        let key = u::put(req).await;

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: None,
                },
            )
            .await?;

        if let Some((embeds, components)) = u::run(&key).await {
            ctx.bot
                .http
                .interaction(ctx.application_id.cast())
                .update_response(&ctx.token)
                .embeds(Some(&embeds))
                .components(Some(&components))
                .await?;
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

        let focused_upper = focused_value.trim().to_uppercase();
        let focused_lower = focused_value.trim().to_lowercase();

        let choices = match focused_name {
            "type" => u::VALID_TYPES
                .iter()
                .filter(|t| t.starts_with(&focused_upper))
                .take(25)
                .map(|t| CommandOptionChoice {
                    name: t.to_string(),
                    name_localizations: None,
                    value: CommandOptionChoiceValue::String(t.to_string()),
                })
                .collect::<Vec<_>>(),
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
