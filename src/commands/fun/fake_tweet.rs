use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{CommandBuilder, StringBuilder, UserBuilder};
use twilight_model::channel::message::component::{
    Component, Container, MediaGallery, MediaGalleryItem, Separator, SeparatorSpacingSize, TextDisplay, UnfurledMediaItem
};
use url::form_urlencoded;

pub struct FakeTweetCommand;

#[async_trait]
impl SlashCommand for FakeTweetCommand {
    fn name(&self) -> &str {
        "fake-tweet"
    }

    fn description(&self) -> &str {
        "Generate a lightweight fake tweet"
    }

    fn build(&self) -> Command {
        CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput)
            .option(StringBuilder::new("tweet", "Text to include in the fake tweet").required(true))
            .option(UserBuilder::new("user", "User to mimic"))
            .build()
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let mut tweet_text: Option<String> = None;
        let mut target_user = ctx.author_id;

        for opt in &ctx.data.options {
            match opt.name.as_str() {
                "tweet" => {
                    if let CommandOptionValue::String(s) = &opt.value {
                        tweet_text = Some(s.clone());
                    }
                }
                "user" => {
                    if let CommandOptionValue::User(id) = opt.value {
                        target_user = Some(id);
                    }
                }
                _ => {}
            }
        }

        let tweet = tweet_text.unwrap_or_else(|| "I love Alya-chan!".to_string());

        let (display_name, username, avatar_url) = if let Some(uid) = target_user {
            match ctx.bot.http.user(uid).await {
                Ok(resp) => match resp.model().await {
                    Ok(user) => {
                        let avatar = user
                            .avatar
                            .as_ref()
                            .map(|h| {
                                format!("https://cdn.discordapp.com/avatars/{}/{}.jpg", uid, h)
                            })
                            .unwrap_or_default();
                        (
                            user.global_name
                                .clone()
                                .unwrap_or_else(|| user.name.clone()),
                            user.name,
                            avatar,
                        )
                    }
                    Err(_) => ("Someone".to_string(), "someone".to_string(), String::new()),
                },
                Err(_) => ("Someone".to_string(), "someone".to_string(), String::new()),
            }
        } else {
            ("Someone".to_string(), "someone".to_string(), String::new())
        };

        let enc = |s: &str| -> String { form_urlencoded::byte_serialize(s.as_bytes()).collect() };
        let image_url = format!(
            "https://some-random-api.com/canvas/tweet?avatar={}&displayname={}&username={}&comment={}",
            enc(&avatar_url),
            enc(&display_name),
            enc(&username),
            enc(&tweet),
        );

        let container = Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("## {}'s Tweet", display_name),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: Some(true),
                    spacing: None,
                }),
                Component::MediaGallery(MediaGallery {
                    id: None,
                    items: vec![MediaGalleryItem {
                        media: UnfurledMediaItem{
                            url: image_url.clone(),
                            content_type: None,
                            height: None,
                            width: None,
                            proxy_url: None,
                        },
                        description: Some(tweet.clone()),
                        spoiler: None,
                    }],
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("Requested by {}", username),
                }),
            ],
            accent_color: Some(Some(ctx.bot.config.color.primary)),
            spoiler: None,
        };

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        components: Some(vec![Component::Container(container)]),
                        flags: Some(twilight_model::channel::message::MessageFlags::IS_COMPONENTS_V2),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
