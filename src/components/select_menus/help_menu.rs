use crate::handlers::CommandManager;
use crate::types::{BotResult, ComponentContext, ComponentHandler};
use async_trait::async_trait;
use std::sync::Arc;
use twilight_model::application::interaction::InteractionData;
use twilight_model::channel::message::component::{
    ActionRow, Component, SelectMenu, SelectMenuOption, SelectMenuType,
};
use twilight_model::channel::message::EmojiReactionType;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedBuilder;

pub struct HelpMenuSelect {
    cmd_mgr: Arc<CommandManager>,
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

impl HelpMenuSelect {
    pub fn new(cmd_mgr: Arc<CommandManager>) -> Self {
        Self { cmd_mgr }
    }
}

#[async_trait]
impl ComponentHandler for HelpMenuSelect {
    fn custom_id_pattern(&self) -> &str {
        "guild-helpMenu"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &*ctx.interaction;

        if let Some(InteractionData::MessageComponent(mc)) = &interaction.data {
            let category = mc.values.first().cloned().unwrap_or_default();

            let commands = self.cmd_mgr.get_commands_by_category(&category);

            let mut embeds = Vec::new();
            for chunk in commands.chunks(5) {
                let description = chunk
                    .iter()
                    .map(|cmd| {
                        let meta = cmd.metadata();
                        format!("**/{0}**\n```{1}```", meta.name, meta.description)
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");

                let embed = EmbedBuilder::new()
                    .color(ctx.bot.config.color.primary)
                    .title(format!("{} Commands", capitalize_first(&category)))
                    .description(description)
                    .build();

                embeds.push(embed);
            }

            if embeds.is_empty() {
                let embed = EmbedBuilder::new()
                    .color(ctx.bot.config.color.primary)
                    .title(format!("{} Commands", capitalize_first(&category)))
                    .description("No commands in this category yet.")
                    .build();
                embeds.push(embed);
            }

            let categories = self.cmd_mgr.get_all_categories();
            let options = categories
                .into_iter()
                .map(|c| SelectMenuOption {
                    default: c == category,
                    emoji: {
                        // Map category to emoji string from config
                        let emoji_str = match c {
                            "configurations" => &ctx.bot.config.emoji.pencil,
                            "informations" => &ctx.bot.config.emoji.info,
                            "music" => &ctx.bot.config.emoji.music,
                            "filters" => &ctx.bot.config.emoji.list,
                            "playlists" => &ctx.bot.config.emoji.folder,
                            "reports" => &ctx.bot.config.emoji.warn,
                            _ => &ctx.bot.config.emoji.question,
                        };

                        // Try to parse string into EmojiReactionType
                        fn parse_emoji(s: &str) -> Option<EmojiReactionType> {
                            let s = s.trim();
                            if s.starts_with('<') && s.ends_with('>') {
                                let inner = &s[1..s.len() - 1];
                                let parts: Vec<&str> = inner.split(':').collect();
                                if parts.len() == 3 {
                                    let animated = parts[0] == "a";
                                    let name = parts[1];
                                    if let Ok(id_num) = parts[2].parse::<u64>() {
                                        return Some(EmojiReactionType::Custom {
                                            animated,
                                            id: Id::new(id_num),
                                            name: Some(name.to_string()),
                                        });
                                    }
                                }
                            }
                            // Fallback to unicode emoji (or literal string)
                            if !s.is_empty() {
                                return Some(EmojiReactionType::Unicode {
                                    name: s.to_string(),
                                });
                            }
                            None
                        }

                        parse_emoji(emoji_str)
                    },
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

            // We'll set target page indices in the custom_id as: pagination-pagePrev:{category}:{page}
            let mut components = Vec::new();

            components.push(Component::ActionRow(ActionRow {
                id: None,
                components: vec![Component::SelectMenu(select)],
            }));

            let pages = embeds.len();
            let mut buttons = Vec::new();
            if pages > 1 {
                let prev_custom = format!("pagination-pagePrev:{}:{}", category, 0);
                let next_custom = format!("pagination-pageNext:{}:{}", category, 1);

                buttons.push(Component::Button(
                    twilight_model::channel::message::component::Button {
                        id: None,
                        custom_id: Some(prev_custom),
                        disabled: true,
                        emoji: None,
                        label: Some("Prev".to_string()),
                        style: twilight_model::channel::message::component::ButtonStyle::Secondary,
                        url: None,
                        sku_id: None,
                    },
                ));

                buttons.push(Component::Button(
                    twilight_model::channel::message::component::Button {
                        id: None,
                        custom_id: Some(next_custom),
                        disabled: false,
                        emoji: None,
                        label: Some("Next".to_string()),
                        style: twilight_model::channel::message::component::ButtonStyle::Primary,
                        url: None,
                        sku_id: None,
                    },
                ));

                components.push(Component::ActionRow(ActionRow {
                    id: None,
                    components: buttons,
                }));
            }

            ctx.bot
                .http
                .interaction(interaction.application_id.cast())
                .create_response(
                    interaction.id.cast(),
                    &interaction.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::UpdateMessage,
                        data: Some(InteractionResponseData {
                            embeds: Some(embeds),
                            components: Some(components),
                            ..Default::default()
                        }),
                    },
                )
                .await?;
        }

        Ok(())
    }
}
