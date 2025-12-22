use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use std::sync::Arc;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::util::Timestamp;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder};

pub struct HelpCommand {
    command_manager: Option<Arc<crate::handlers::CommandManager>>,
}

impl HelpCommand {
    pub fn new() -> Self {
        Self {
            command_manager: None,
        }
    }

    pub fn with_manager(mut self, manager: Arc<crate::handlers::CommandManager>) -> Self {
        self.command_manager = Some(manager);
        self
    }
}

#[async_trait]
impl SlashCommand for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }

    fn description(&self) -> &str {
        "Show all available commands organized by category"
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        ctx.bot
            .http
            .interaction(ctx.interaction_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::DeferredChannelMessageWithSource,
                    data: None,
                },
            )
            .await?;

        let mut embeds = Vec::new();

        if let Some(manager) = &self.command_manager {
            let categories = manager.get_all_categories();

            if categories.is_empty() {
                let embed = EmbedBuilder::new()
                    .title("Alya-chan Help Menu")
                    .description("No commands registered yet!")
                    .color(0x9b59b6)
                    .build();
                embeds.push(embed);
            } else {
                for category in categories {
                    let commands = manager.get_commands_by_category(category);

                    if commands.is_empty() {
                        continue;
                    }

                    let mut fields = Vec::new();
                    for cmd in commands {
                        let metadata = cmd.metadata();
                        let field_value = format!("`/{}`\n{}", metadata.name, metadata.description);
                        fields.push(EmbedFieldBuilder::new(&metadata.name, field_value));
                    }

                    let mut embed_builder = EmbedBuilder::new()
                        .title(format!("📚 {} Commands", capitalize_first(category)))
                        .color(0x9b59b6);

                    for field in fields {
                        embed_builder = embed_builder.field(field);
                    }

                    embeds.push(embed_builder.build());
                }

                let footer_embed = EmbedBuilder::new()
                    .title("💜 Alya-chan")
                    .description("Discord multipurpose bot made with Twilight\n\nUse `/` followed by command name to execute commands")
                    .color(0x9b59b6)
                    .timestamp(Timestamp::from_secs(chrono::Utc::now().timestamp()).unwrap())
                    .build();
                embeds.push(footer_embed);
            }
        }

        ctx.bot
            .http
            .interaction(ctx.interaction_id.cast())
            .create_followup(&ctx.token)
            .embeds(if embeds.is_empty() { &[] } else { &embeds })
            .await?;

        Ok(())
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

impl Default for HelpCommand {
    fn default() -> Self {
        Self::new()
    }
}
