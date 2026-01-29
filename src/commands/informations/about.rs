use crate::config::EmojiConfig;
use crate::types::{
    BotResult, ComponentContext, ComponentHandler, SlashCommand, SlashCommandContext,
};
use async_trait::async_trait;
use tokio::time::{sleep, Duration};
use twilight_model::channel::message::component::{
    ActionRow, Button, ButtonStyle, Component, Container, Separator, SeparatorSpacingSize,
    TextDisplay,
};
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

pub struct AboutCommand;

impl AboutCommand {
    fn create_about_container(color: u32, emoji: &EmojiConfig) -> Container {
        Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("{} **About Alya-chan**", emoji.ribbon),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: Some(true),
                    spacing: None,
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "{} Hmph. Fine, pay attention.\n\n\
                        I am **Alya-chan**, a highly proficient, multipurpose Discord bot. \
                        I keep your server from chaos and handle **moderation**, **utility**, and **fun** tasks with ease.",
                        emoji.info,
                    ),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "{} **Inspiration**\n\
                        Inspired by **Alisa Mikhailovna Kujou**, my capabilities are top-tier. \
                        If you slip up, I'll probably mumble **\"Ты дурак?\"** (Are you an idiot?) under my breath—but I'll fix it anyway.",
                        emoji.info,
                    ),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "{} **Quick Links**\n\
                        {} [Invite Me](https://discord.com/oauth2/authorize?client_id=1260252174861074442)\n\
                        {} [Support Server](https://discord.gg/pTbFUFdppU)\n\
                        {} [Vote for me](https://top.gg/bot/1260252174861074442/vote)",
                        emoji.link,
                        emoji.link,
                        emoji.chat,
                        emoji.heart,
                    ),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "{} **Credits**\n\
                        {} Creator: iaMJ\n\
                        {} Developed by: Tronix Development\n\
                        {} Country: Indonesia",
                        emoji.user,
                        emoji.user,
                        emoji.office,
                        emoji.world,
                    ),
                }),
            ],
            accent_color: Some(Some(color)),
            spoiler: None,
        }
    }

    fn create_contributors_container(color: u32, emoji: &EmojiConfig) -> Container {
        Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("{} **Contributors & Credits**", emoji.user),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: Some(true),
                    spacing: None,
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "{} **Special Thanks**\n\n\
                        {} **iaMJ** - Creator of Alya\n\
                        {} Tronix Development & community supporters\n\n\
                        Thank you for making this project possible!",
                        emoji.info,
                        emoji.heart,
                        emoji.heart,
                    ),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "{} **License Information**\n\n\
                        {} Licensed under **GNU Affero General Public License v3.0**\n\
                        {} [View Full License](https://github.com/idMJA/Alya-chan/blob/master/LICENSE)",
                        emoji.link,
                        emoji.info,
                        emoji.link,
                    ),
                }),
            ],
            accent_color: Some(Some(color)),
            spoiler: None,
        }
    }

    fn create_packages_container(color: u32, emoji: &EmojiConfig) -> Container {
        Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("{} **Tech Stack**", emoji.settings),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: Some(true),
                    spacing: None,
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "{} **Core Libraries**\n\n\
                        {} **[Twilight](https://twilight.rs)** - Discord API library\n\
                        {} **[Tokio](https://tokio.rs)** - Async runtime\n\
                        {} **[libsql](https://github.com/tursodatabase/libsql)** - SQLite + Turso\n\
                        {} **[Tracing](https://github.com/tokio-rs/tracing)** - Structured logging",
                        emoji.list,
                        emoji.twilight,
                        emoji.tokio,
                        emoji.libsql,
                        emoji.tracing,
                    ),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("{} **Powered by Rust**\n\nBuilt with modern async patterns and memory safety.", emoji.robot),
                }),
            ],
            accent_color: Some(Some(color)),
            spoiler: None,
        }
    }

    fn create_button_row(active_button: &str, expired: bool) -> Component {
        let button_about = Component::Button(Button {
            custom_id: Some("about_btn_about".to_owned()),
            disabled: expired || active_button == "about",
            label: Some("About".to_owned()),
            style: if expired {
                ButtonStyle::Secondary
            } else if active_button == "about" {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Secondary
            },
            emoji: None,
            url: None,
            id: None,
            sku_id: None,
        });

        let button_contributors = Component::Button(Button {
            custom_id: Some("about_btn_contributors".to_owned()),
            disabled: expired || active_button == "contributors",
            label: Some("Contributors".to_owned()),
            style: if expired {
                ButtonStyle::Secondary
            } else if active_button == "contributors" {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Secondary
            },
            emoji: None,
            url: None,
            id: None,
            sku_id: None,
        });

        let button_packages = Component::Button(Button {
            custom_id: Some("about_btn_packages".to_owned()),
            disabled: expired || active_button == "packages",
            label: Some("Packages".to_owned()),
            style: if expired {
                ButtonStyle::Secondary
            } else if active_button == "packages" {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Secondary
            },
            emoji: None,
            url: None,
            id: None,
            sku_id: None,
        });

        Component::ActionRow(ActionRow {
            id: None,
            components: vec![button_about, button_contributors, button_packages],
        })
    }
}

#[async_trait]
impl SlashCommand for AboutCommand {
    fn name(&self) -> &str {
        "about"
    }

    fn description(&self) -> &str {
        "Show information about Alya-chan"
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let color = ctx.bot.config.color.primary;
        let emoji = &ctx.bot.config.emoji;
        let container = Self::create_about_container(color, emoji);
        let button_row = Self::create_button_row("about", false);

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        content: None,
                        components: Some(vec![Component::Container(container), button_row]),
                        flags: Some(MessageFlags::IS_COMPONENTS_V2),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        let http = ctx.bot.http.clone();
        let token = ctx.token.clone();
        let application_id = ctx.application_id;
        let emoji = emoji.clone();
        tokio::spawn(async move {
            sleep(Duration::from_secs(60)).await;

            let container = AboutCommand::create_about_container(color, &emoji);
            let button_row = AboutCommand::create_button_row("about", true);
            let components = vec![Component::Container(container), button_row];

            let _ = http
                .interaction(application_id.cast())
                .update_response(&token)
                .components(Some(&components))
                .flags(MessageFlags::IS_COMPONENTS_V2)
                .await;
        });

        Ok(())
    }
}

pub struct AboutButton;

#[async_trait]
impl ComponentHandler for AboutButton {
    fn custom_id_pattern(&self) -> &str {
        "about_btn_*"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &ctx.interaction;

        let application_id = interaction.application_id;
        let interaction_id = interaction.id;
        let token = interaction.token.clone();

        let custom_id = match &interaction.data {
            Some(twilight_model::application::interaction::InteractionData::MessageComponent(
                data,
            )) => data.custom_id.clone(),
            _ => return Ok(()),
        };

        let color = ctx.bot.config.color.primary;
        let emoji = &ctx.bot.config.emoji;
        let button_id = custom_id.strip_prefix("about_btn_").unwrap_or("");

        let (container, active_tab) = match button_id {
            "about" => (AboutCommand::create_about_container(color, emoji), "about"),
            "contributors" => (
                AboutCommand::create_contributors_container(color, emoji),
                "contributors",
            ),
            "packages" => (
                AboutCommand::create_packages_container(color, emoji),
                "packages",
            ),
            _ => return Ok(()),
        };

        let button_row = AboutCommand::create_button_row(active_tab, false);

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
                        components: Some(vec![Component::Container(container), button_row]),
                        flags: Some(MessageFlags::IS_COMPONENTS_V2),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
