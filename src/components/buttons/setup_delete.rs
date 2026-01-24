use crate::database::service::AlyaDatabase;
use crate::types::{error::BotError, BotResult, ComponentContext, ComponentHandler};
use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use twilight_model::application::interaction::InteractionData;
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

pub struct SetupDeleteButton;

impl SetupDeleteButton {
    fn build_static_row(label: &str) -> Component {
        Component::ActionRow(ActionRow {
            id: None,
            components: vec![Component::Button(Button {
                custom_id: None,
                disabled: true,
                label: Some(label.to_string()),
                style: ButtonStyle::Secondary,
                emoji: None,
                url: None,
                id: None,
                sku_id: None,
            })],
        })
    }

    fn build_action_row(
        custom_id_confirm: &str,
        confirm_label: &str,
        custom_id_cancel: &str,
        cancel_label: &str,
    ) -> Component {
        let confirm_btn = Component::Button(Button {
            custom_id: Some(custom_id_confirm.to_string()),
            disabled: false,
            label: Some(confirm_label.to_string()),
            style: ButtonStyle::Danger,
            emoji: None,
            url: None,
            id: None,
            sku_id: None,
        });

        let cancel_btn = Component::Button(Button {
            custom_id: Some(custom_id_cancel.to_string()),
            disabled: false,
            label: Some(cancel_label.to_string()),
            style: ButtonStyle::Secondary,
            emoji: None,
            url: None,
            id: None,
            sku_id: None,
        });

        Component::ActionRow(ActionRow {
            id: None,
            components: vec![confirm_btn, cancel_btn],
        })
    }

    async fn delete_chatbot(guild_id: &str) -> BotResult<bool> {
        if let Ok(db) = AlyaDatabase::get() {
            db.delete_chatbot_setup(guild_id)
                .await
                .map_err(|e| BotError::Other(e.to_string()))?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn delete_globalchat(ctx: &ComponentContext, guild_id: &str) -> BotResult<bool> {
        // Call remote API if configured
        if let Some(gc) = &ctx.bot.config.global_chat {
            let client = reqwest::Client::new();
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
            if let Some(key) = &gc.api_key {
                headers.insert(AUTHORIZATION, format!("Bearer {}", key).parse().unwrap());
            }

            let _ = client
                .delete(format!("{}/remove/{}", gc.api_url, guild_id))
                .headers(headers)
                .send()
                .await;
        }

        if let Ok(db) = AlyaDatabase::get() {
            db.delete_global_chat_channel(guild_id)
                .await
                .map_err(|e| BotError::Other(e.to_string()))?;
            return Ok(true);
        }
        Ok(false)
    }
}

#[async_trait]
impl ComponentHandler for SetupDeleteButton {
    fn custom_id_pattern(&self) -> &str {
        "setup_del_*"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &ctx.interaction;
        let application_id = interaction.application_id;
        let interaction_id = interaction.id;
        let token = interaction.token.clone();

        let custom_id = match &interaction.data {
            Some(InteractionData::MessageComponent(data)) => data.custom_id.clone(),
            _ => return Ok(()),
        };

        // Expected formats:
        // setup_del_chatbot_confirm:<guild_id>
        // setup_del_chatbot_cancel
        // setup_del_globalchat_confirm:<guild_id>
        // setup_del_globalchat_cancel
        let mut parts = custom_id.split(':');
        let action = parts.next().unwrap_or("");
        let guild_id = parts.next();

        let emoji_yes = &ctx.bot.config.emoji.yes;
        let emoji_no = &ctx.bot.config.emoji.no;

        let (content, components) = match action {
            "setup_del_chatbot_confirm" => {
                if let Some(gid) = guild_id {
                    let success = Self::delete_chatbot(gid).await.unwrap_or(false);
                    let msg = if success {
                        format!("{} Chatbot setup deleted.", emoji_yes)
                    } else {
                        format!("{} Failed to delete chatbot setup.", emoji_no)
                    };
                    (msg, vec![])
                } else {
                    (format!("{} Invalid request.", emoji_no), vec![])
                }
            }
            "setup_del_chatbot_cancel" => {
                (format!("{} Cancellation acknowledged.", emoji_no), vec![])
            }
            "setup_del_globalchat_confirm" => {
                if let Some(gid) = guild_id {
                    let success = Self::delete_globalchat(ctx, gid).await.unwrap_or(false);
                    let msg = if success {
                        format!("{} Global chat setup deleted.", emoji_yes)
                    } else {
                        format!("{} Failed to delete global chat setup.", emoji_no)
                    };
                    (msg, vec![])
                } else {
                    (format!("{} Invalid request.", emoji_no), vec![])
                }
            }
            "setup_del_globalchat_cancel" => {
                (format!("{} Cancellation acknowledged.", emoji_no), vec![])
            }
            _ => return Ok(()),
        };

        ctx.bot
            .http
            .interaction(application_id.cast())
            .create_response(
                interaction_id.cast(),
                &token,
                &InteractionResponse {
                    kind: InteractionResponseType::UpdateMessage,
                    data: Some(InteractionResponseData {
                        content: Some(content),
                        components: Some(components),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
