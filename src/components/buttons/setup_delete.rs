use crate::database::service::AlyaDatabase;
use crate::types::{error::BotError, BotResult, ComponentContext, ComponentHandler};
use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use twilight_model::application::interaction::InteractionData;
use twilight_model::channel::message::component::{
    Component, Container, Separator, SeparatorSpacingSize, TextDisplay,
};
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

pub struct SetupDeleteButton;

impl SetupDeleteButton {
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
                headers.insert(AUTHORIZATION, format!("Bearer {key}").parse().unwrap());
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
    fn custom_id_pattern(&self) -> &'static str {
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

        let (container, components) = match action {
            "setup_del_chatbot_confirm" => {
                if let Some(gid) = guild_id {
                    let success = Self::delete_chatbot(gid).await.unwrap_or(false);
                    let content = if success {
                        format!("{emoji_yes} Chatbot setup deleted.")
                    } else {
                        format!("{emoji_no} Failed to delete chatbot setup.")
                    };
                    (build_status_container("Chatbot Setup", &content), vec![])
                } else {
                    (
                        build_status_container(
                            "Chatbot Setup",
                            &format!("{emoji_no} Invalid request."),
                        ),
                        vec![],
                    )
                }
            }
            "setup_del_chatbot_cancel" => (
                build_status_container(
                    "Chatbot Setup",
                    &format!("{emoji_no} Cancellation acknowledged."),
                ),
                vec![],
            ),
            "setup_del_globalchat_confirm" => {
                if let Some(gid) = guild_id {
                    let success = Self::delete_globalchat(ctx, gid).await.unwrap_or(false);
                    let content = if success {
                        format!("{emoji_yes} Global chat setup deleted.")
                    } else {
                        format!("{emoji_no} Failed to delete global chat setup.")
                    };
                    (build_status_container("Global Chat", &content), vec![])
                } else {
                    (
                        build_status_container(
                            "Global Chat",
                            &format!("{emoji_no} Invalid request."),
                        ),
                        vec![],
                    )
                }
            }
            "setup_del_globalchat_cancel" => (
                build_status_container(
                    "Global Chat",
                    &format!("{emoji_no} Cancellation acknowledged."),
                ),
                vec![],
            ),
            _ => return Ok(()),
        };

        let components = if components.is_empty() {
            vec![Component::Container(container)]
        } else {
            let mut out = vec![Component::Container(container)];
            out.extend(components);
            out
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
                        content: None,
                        components: Some(components),
                        flags: Some(MessageFlags::IS_COMPONENTS_V2),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}

fn build_status_container(title: &str, content: &str) -> Container {
    Container {
        id: None,
        components: vec![
            Component::TextDisplay(TextDisplay {
                id: None,
                content: format!("## {title}\n{content}"),
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
