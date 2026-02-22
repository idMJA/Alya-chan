use crate::database::service::AlyaDatabase;
use crate::types::{error::BotError, BotResult, ComponentContext, ComponentHandler};
use async_trait::async_trait;
use twilight_model::application::interaction::InteractionData;
use twilight_model::channel::message::component::{
    Component, Container, Separator, SeparatorSpacingSize, TextDisplay,
};
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::permission_overwrite::{PermissionOverwrite, PermissionOverwriteType};
use twilight_model::channel::ChannelType;
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

pub struct ChatbotCreateButton;

#[async_trait]
impl ComponentHandler for ChatbotCreateButton {
    fn custom_id_pattern(&self) -> &'static str {
        "chatbot_create:*"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &ctx.interaction;
        let interaction_id = interaction.id;
        let token = interaction.token.clone();

        let custom_id = match &interaction.data {
            Some(InteractionData::MessageComponent(data)) => data.custom_id.clone(),
            _ => return Ok(()),
        };

        let mut parts = custom_id.split(':');
        parts.next();
        let guild_id = if let Some(id) = parts.next().and_then(|id| id.parse::<u64>().ok()) {
            twilight_model::id::Id::<twilight_model::id::marker::GuildMarker>::new(id)
        } else {
            let container = build_status_container(
                "Chatbot Setup",
                &format!("{} Invalid guild ID.", ctx.bot.config.emoji.no),
            );
            return update_message(ctx, interaction_id, &token, container, vec![]).await;
        };

        let bot_id = ctx.bot.bot_user.id;

        let everyone_role_id = guild_id;

        let permission_overwrites = vec![
            PermissionOverwrite {
                id: bot_id.cast(),
                kind: PermissionOverwriteType::Member,
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::READ_MESSAGE_HISTORY
                    | Permissions::MANAGE_MESSAGES,
                deny: Permissions::empty(),
            },
            PermissionOverwrite {
                id: everyone_role_id.cast(),
                kind: PermissionOverwriteType::Role,
                allow: Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
            },
        ];

        let new_channel = ctx
            .bot
            .http
            .create_guild_channel(guild_id, "🤖・chatbot")
            .kind(ChannelType::GuildText)
            .topic("Chatbot responses and interactions")
            .permission_overwrites(&permission_overwrites)
            .await
            .map_err(|e| BotError::Other(e.to_string()))?
            .model()
            .await
            .map_err(|e| BotError::Other(e.to_string()))?;

        if let Ok(db) = AlyaDatabase::get() {
            db.create_chatbot_setup(
                &guild_id.get().to_string(),
                &new_channel.id.get().to_string(),
            )
            .await
            .map_err(|e| BotError::Other(e.to_string()))?;
        } else {
            let container = build_status_container(
                "Chatbot Setup",
                &format!("{} Database not ready.", ctx.bot.config.emoji.no),
            );
            return update_message(ctx, interaction_id, &token, container, vec![]).await;
        }

        let success_container = build_status_container(
            "Chatbot Setup",
            &format!(
                "{} Setup complete.\n{} Channel: <#{}>",
                ctx.bot.config.emoji.yes, ctx.bot.config.emoji.folder, new_channel.id
            ),
        );

        update_message(ctx, interaction_id, &token, success_container, vec![]).await
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

async fn update_message(
    ctx: &ComponentContext,
    interaction_id: twilight_model::id::Id<twilight_model::id::marker::InteractionMarker>,
    token: &str,
    container: Container,
    components: Vec<Component>,
) -> BotResult<()> {
    let components = if components.is_empty() {
        vec![Component::Container(container)]
    } else {
        let mut out = vec![Component::Container(container)];
        out.extend(components);
        out
    };

    ctx.bot
        .http
        .interaction(ctx.interaction.application_id.cast())
        .create_response(
            interaction_id.cast(),
            token,
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
