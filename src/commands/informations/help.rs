use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use std::sync::Arc;
use twilight_model::channel::message::component::{
    ActionRow, Component, SelectMenu, SelectMenuOption, SelectMenuType,
};
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::util::Timestamp;
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

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

        if let Some(manager) = &self.command_manager {
            let categories = manager.get_all_categories();

            let author_mention = ctx
                .author_id
                .as_ref()
                .map(|id| format!("<@{}>", id))
                .unwrap_or_else(|| String::from("<@unknown>"));

            let mut main = EmbedBuilder::new()
                .color(ctx.bot.config.color.primary)
                .title("Alya-chan Help Center")
                .description(format!("**Konnichiwa! {}, I'm Alya-chan**\n\n**A multifunctional Discord bot inspired by your favorite anime characters. With powerful features, Alya-chan is not only ready to accompany you to play, but also help you manage your Discord server more effectively. Equipped with various moderation, entertainment, and utility features, and more. Alya-chan is a loyal friend who is ready to help anytime!**", author_mention))
                .field(twilight_util::builder::embed::EmbedFieldBuilder::new("\u{200B}", "\u{200B}"))   
                .field(twilight_util::builder::embed::EmbedFieldBuilder::new(
                    "Categories",
                    categories
                        .iter()
                        .map(|c| {
                            let emoji = match *c {
                                "configurations" => &ctx.bot.config.emoji.pencil,
                                "informations" => &ctx.bot.config.emoji.info,
                                "music" => &ctx.bot.config.emoji.music,
                                "filters" => &ctx.bot.config.emoji.list,
                                "playlists" => &ctx.bot.config.emoji.folder,
                                "reports" => &ctx.bot.config.emoji.warn,
                                _ => &ctx.bot.config.emoji.question,
                            };

                            format!("{} : **{}**", emoji, capitalize_first(c))
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                ))
                .timestamp(Timestamp::from_secs(chrono::Utc::now().timestamp()).unwrap());

            let banner = &ctx.bot.config.info.banner;
            main = main.image(ImageSource::url(banner).expect("banner validated at startup"));

            main = main.footer(twilight_util::builder::embed::EmbedFooterBuilder::new(
                "Thanks for choosing Alya-chan!",
            ));

            let main_embed = main.build();

            let options = categories
                .into_iter()
                .map(|c| SelectMenuOption {
                    default: false,
                    emoji: None,
                    description: None,
                    label: capitalize_first(c),
                    value: c.to_string(),
                })
                .collect::<Vec<_>>();

            let select = SelectMenu {
                id: None,
                channel_types: None,
                custom_id: "guild-helpMenu".to_string(),
                default_values: None,
                disabled: false,
                kind: SelectMenuType::Text,
                max_values: Some(1),
                min_values: Some(1),
                options: Some(options),
                placeholder: Some("Select a category".to_string()),
                required: None,
            };

            let components = vec![Component::ActionRow(ActionRow {
                id: None,
                components: vec![Component::SelectMenu(select)],
            })];

            ctx.bot
                .http
                .interaction(ctx.application_id.cast())
                .create_followup(&ctx.token)
                .embeds(&[main_embed])
                .components(&components)
                .await?;
        }

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
