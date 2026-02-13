use crate::handlers::CommandManager;
use crate::types::{BotResult, ComponentContext, ComponentHandler};
use async_trait::async_trait;
use std::sync::Arc;
use twilight_model::application::interaction::InteractionData;
use twilight_model::channel::message::component::{ActionRow, Component};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::embed::EmbedBuilder;

pub struct PaginatorButton {
    cmd_mgr: Arc<CommandManager>,
}

impl PaginatorButton {
    pub const fn new(cmd_mgr: Arc<CommandManager>) -> Self {
        Self { cmd_mgr }
    }
}

#[async_trait]
impl ComponentHandler for PaginatorButton {
    fn custom_id_pattern(&self) -> &'static str {
        "pagination-page"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &*ctx.interaction;

        if let Some(InteractionData::MessageComponent(mc)) = &interaction.data {
            let custom = mc.custom_id.as_str();

            let parts: Vec<&str> = custom.split(':').collect();
            if parts.len() != 3 {
                return Ok(());
            }

            let _action = parts[0..2].join(":");
            let category = parts[1];
            let page: usize = parts[2].parse().unwrap_or(0);

            let commands = self.cmd_mgr.get_commands_by_category(category);

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
                    .title(format!("{} Commands", category))
                    .description(description)
                    .build();

                embeds.push(embed);
            }

            let pages = embeds.len();
            let mut components = Vec::new();

            let categories = self.cmd_mgr.get_all_categories();
            let options = categories
                .into_iter()
                .map(
                    |c| twilight_model::channel::message::component::SelectMenuOption {
                        default: c == category,
                        emoji: None,
                        description: None,
                        label: c.to_string(),
                        value: c.to_string(),
                    },
                )
                .collect::<Vec<_>>();

            let select = twilight_model::channel::message::component::SelectMenu {
                id: None,
                channel_types: None,
                custom_id: "guild-helpMenu".to_string(),
                default_values: None,
                disabled: false,
                kind: twilight_model::channel::message::component::SelectMenuType::Text,
                max_values: Some(1),
                min_values: Some(1),
                options: Some(options),
                placeholder: Some("Select a category".to_string()),
                required: None,
            };

            components.push(Component::ActionRow(ActionRow {
                id: None,
                components: vec![Component::SelectMenu(select)],
            }));

            if pages > 1 {
                let prev_page = if page == 0 { 0 } else { page - 1 };
                let next_page = if page + 1 >= pages {
                    pages - 1
                } else {
                    page + 1
                };

                let prev_custom = format!("pagination-pagePrev:{}:{}", category, prev_page);
                let next_custom = format!("pagination-pageNext:{}:{}", category, next_page);

                let buttons = vec![
                    Component::Button(twilight_model::channel::message::component::Button {
                        id: None,
                        custom_id: Some(prev_custom),
                        disabled: page == 0,
                        emoji: None,
                        label: Some("Prev".to_string()),
                        style: twilight_model::channel::message::component::ButtonStyle::Secondary,
                        url: None,
                        sku_id: None,
                    }),
                    Component::Button(twilight_model::channel::message::component::Button {
                        id: None,
                        custom_id: Some(next_custom),
                        disabled: page + 1 >= pages,
                        emoji: None,
                        label: Some("Next".to_string()),
                        style: twilight_model::channel::message::component::ButtonStyle::Primary,
                        url: None,
                        sku_id: None,
                    }),
                ];

                components.push(Component::ActionRow(ActionRow {
                    id: None,
                    components: buttons,
                }));
            }

            let embed_to_send = embeds.get(page).cloned();
            ctx.bot
                .http
                .interaction(interaction.application_id.cast())
                .create_response(
                    interaction.id.cast(),
                    &interaction.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::UpdateMessage,
                        data: Some(InteractionResponseData {
                            embeds: embed_to_send.map(|e| vec![e]),
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
