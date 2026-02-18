use crate::types::{AutocompleteContext, BotResult, SlashCommand, SlashCommandContext};
use crate::utils::whois;
use async_trait::async_trait;
use twilight_model::application::command::{
    Command, CommandOptionChoice, CommandOptionChoiceValue, CommandType,
};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{CommandBuilder, StringBuilder};

pub struct WhoisCommand;

#[async_trait]
impl SlashCommand for WhoisCommand {
    fn name(&self) -> &'static str {
        "whois"
    }

    fn description(&self) -> &'static str {
        "Perform a WHOIS lookup for a domain, IP or ASN"
    }

    fn build(&self) -> Command {
        CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput)
            .option(
                StringBuilder::new("query", "The domain name, IP address or ASN to lookup")
                    .required(true)
                    .autocomplete(true),
            )
            .build()
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let mut query = None;

        for opt in &ctx.data.options {
            if opt.name == "query" {
                if let CommandOptionValue::String(s) = &opt.value {
                    query = Some(s.clone());
                }
            }
        }

        let query = query.unwrap_or_default();

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

        match whois::who(&query).await {
            Ok(embed) => {
                ctx.bot
                    .http
                    .interaction(ctx.application_id.cast())
                    .update_response(&ctx.token)
                    .embeds(Some(&[embed]))
                    .await?;
            }
            Err(err) => {
                ctx.bot
                    .http
                    .interaction(ctx.application_id.cast())
                    .update_response(&ctx.token)
                    .content(Some(&err))
                    .await?;
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

        let choices = if focused_name == "query" {
            let trimmed = focused_value.trim();
            if trimmed.is_empty() {
                vec![
                    CommandOptionChoice {
                        name: "example.com".to_string(),
                        name_localizations: None,
                        value: CommandOptionChoiceValue::String("example.com".to_string()),
                    },
                    CommandOptionChoice {
                        name: "1.1.1.1".to_string(),
                        name_localizations: None,
                        value: CommandOptionChoiceValue::String("1.1.1.1".to_string()),
                    },
                    CommandOptionChoice {
                        name: "AS13335".to_string(),
                        name_localizations: None,
                        value: CommandOptionChoiceValue::String("AS13335".to_string()),
                    },
                ]
            } else {
                let digits = trimmed.trim_start_matches("as").trim_start_matches("AS");
                if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                    vec![CommandOptionChoice {
                        name: format!("AS{digits}"),
                        name_localizations: None,
                        value: CommandOptionChoiceValue::String(format!("AS{digits}")),
                    }]
                } else {
                    vec![CommandOptionChoice {
                        name: trimmed.to_string(),
                        name_localizations: None,
                        value: CommandOptionChoiceValue::String(trimmed.to_string()),
                    }]
                }
            }
        } else {
            Vec::new()
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
