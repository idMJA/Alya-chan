use crate::config::EmojiConfig;
use crate::types::{
    BotResult, ComponentContext, ComponentHandler, SlashCommand, SlashCommandContext,
};
use async_trait::async_trait;
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::embed::EmbedBuilder;

pub struct AboutCommand;

impl AboutCommand {
    fn create_about_embed(color: u32, emoji: &EmojiConfig) -> EmbedBuilder {
        EmbedBuilder::new()
            .color(color)
            .title("About Alya")
            .description(
                format!(
                    "{info} **About Me**\n\n\
                    {info} Hmph. Fine, pay attention. I am **Alya-chan**, a highly proficient, multipurpose Discord bot. I keep your server from chaos and handle **moderation**, **utility**, and **fun** tasks with ease.\n\n\
                    {info} Inspired by **Alisa Mikhailovna Kujou**, my capabilities are top-tier. If you slip up, I'll probably mumble **\"Ты дурак?\"** (Are you an idiot?) under my breath—but I'll fix it anyway.\n\n\
                    {link} **Links**\n\
                    - [Invite Me](https://discord.com/oauth2/authorize?client_id=1260252174861074442)\n\
                    - [Support Server](https://discord.gg/pTbFUFdppU)\n\
                    - [Vote for me](https://top.gg/bot/1260252174861074442/vote)\n\n\
                    {info} **Credits**\n\
                    - Contributed by: iaMJ\n\
                    - Developed by: Tronix Development\n\
                    - Country: Indonesia",
                    info = emoji.info,
                    link = emoji.link,
                ),
            )
    }

    fn create_contributors_embed(color: u32, emoji: &EmojiConfig) -> EmbedBuilder {
        EmbedBuilder::new()
            .color(color)
            .title("Alya Contributors")
            .description(format!(
                "{info} **Special Thanks**\n\n\
                    - iaMJ: Creator of Alya\n\
                    - Tronix Development & community supporters\n\n\
                    {link} **License**\n\
                    This project is licensed under the GNU Affero General Public License v3.0\n\
                    [View License](https://github.com/idMJA/Alya-chan/blob/master/LICENSE)",
                info = emoji.info,
                link = emoji.link,
            ))
    }

    fn create_packages_embed(color: u32, emoji: &EmojiConfig) -> EmbedBuilder {
        EmbedBuilder::new()
            .color(color)
            .title("Alya Packages & Runtime")
            .description(format!(
                "{list} **Core Libraries**\n\n\
                    - **[Twilight](https://twilight.rs)** - Discord API library\n\
                    - **[Tokio](https://tokio.rs)** - Async runtime\n\
                    - **[libsql](https://github.com/tursodatabase/libsql)** - SQLite + Turso\n\
                    - **[Tracing](https://github.com/tokio-rs/tracing)** - Structured logging",
                list = emoji.list,
            ))
    }

    fn create_button_row(active_button: &str) -> Component {
        // Create buttons using Components v2 structure with proper styling
        let button_about = Component::Button(Button {
            custom_id: Some("about_btn_about".to_owned()),
            disabled: active_button == "about",
            label: Some("About".to_owned()),
            style: if active_button == "about" {
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
            disabled: active_button == "contributors",
            label: Some("Contributors".to_owned()),
            style: if active_button == "contributors" {
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
            disabled: active_button == "packages",
            label: Some("Packages".to_owned()),
            style: if active_button == "packages" {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Secondary
            },
            emoji: None,
            url: None,
            id: None,
            sku_id: None,
        });

        // Wrap buttons into ActionRow (Components v2)
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
        let about_embed = Self::create_about_embed(color, emoji).build();
        let button_row = Self::create_button_row("about");

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
                        embeds: Some(vec![about_embed]),
                        components: Some(vec![button_row]),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}

// Button handler integrated in the same file (like TypeScript version)
pub struct AboutButton;

#[async_trait]
impl ComponentHandler for AboutButton {
    fn custom_id_pattern(&self) -> &str {
        "about_btn_*"
    }

    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        let interaction = &ctx.interaction;

        // Extract data from interaction
        let application_id = interaction.application_id;
        let interaction_id = interaction.id;
        let token = interaction.token.clone();

        // Get custom_id from message component data
        let custom_id = match &interaction.data {
            Some(twilight_model::application::interaction::InteractionData::MessageComponent(
                data,
            )) => data.custom_id.clone(),
            _ => return Ok(()),
        };

        let color = ctx.bot.config.color.primary;
        let emoji = &ctx.bot.config.emoji;
        let button_id = custom_id.strip_prefix("about_btn_").unwrap_or("");

        let (embed, active_tab) = match button_id {
            "about" => (
                AboutCommand::create_about_embed(color, emoji).build(),
                "about",
            ),
            "contributors" => (
                AboutCommand::create_contributors_embed(color, emoji).build(),
                "contributors",
            ),
            "packages" => (
                AboutCommand::create_packages_embed(color, emoji).build(),
                "packages",
            ),
            _ => return Ok(()),
        };

        let button_row = AboutCommand::create_button_row(active_tab);

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
                        embeds: Some(vec![embed]),
                        components: Some(vec![button_row]),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
