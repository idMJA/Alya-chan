use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::message::component::{
    Component, Container, Separator, SeparatorSpacingSize, TextDisplay,
};
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{CommandBuilder, UserBuilder};
use uuid::Uuid;

pub struct HackCommand;

#[allow(clippy::too_many_lines)]
#[async_trait]
impl SlashCommand for HackCommand {
    fn name(&self) -> &'static str {
        "hack"
    }

    fn description(&self) -> &'static str {
        "Hack the mentioned user"
    }

    fn build(&self) -> Command {
        CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput)
            .option(UserBuilder::new("user", "The mentioned user will get hacked.").required(true))
            .build()
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let mut target_user = None;
        for opt in &ctx.data.options {
            if opt.name == "user" {
                if let CommandOptionValue::User(id) = opt.value {
                    target_user = Some(id);
                }
            }
        }

        let target_name = if let Some(uid) = target_user {
            match ctx.bot.http.user(uid).await {
                Ok(resp) => match resp.model().await {
                    Ok(user) => user.global_name.clone().unwrap_or(user.name),
                    Err(_) => "someone".to_string(),
                },
                Err(_) => "someone".to_string(),
            }
        } else {
            "someone".to_string()
        };

        let pick = |arr: &'static [&'static str], seed: u128| arr[(seed as usize) % arr.len()];
        let seed = Uuid::new_v4().as_u128();

        let device_passwords: &'static [&'static str] = &[
            "P@ssw0rd!",
            "letmein",
            "admin123",
            "hunter2",
            "superman007",
            "1234567890",
            "qwertyuiop",
            "password",
            "iloveyou",
            "passw0rd",
            "sg457DS3Sd",
            "YellowDonkey24",
            "987654321",
            "football",
            "dragon",
            "monkey",
            "sunshine",
            "princess",
            "welcome",
            "trustno1",
        ];
        let system_ids: &'static [&'static str] = &[
            "ID-001", "ID-002", "ID-003", "ID-004", "ID-005", "ID-006", "ID-007", "US-9981",
            "UK-5542", "JP-1123", "RU-7788", "DE-3344", "FR-2211", "BR-9900",
        ];
        let wifi_names: &'static [&'static str] = &[
            "Rumah123",
            "Indihome",
            "Kostan",
            "CafeWifi",
            "MyWifi",
            "PublicNet",
            "SecretNet",
            "Starbucks_Free",
            "McDonalds_Guest",
            "Airport_WiFi",
            "HotelHilton",
            "TokyoNet",
            "LondonWifi",
            "NYC_FreeWiFi",
        ];
        let wifi_pw: &'static [&'static str] = &[
            "password123",
            "qwerty123",
            "wifi2025",
            "supersecret",
            "letmein",
            "hackme",
            "12345678",
            "welcome2025",
            "freewifi",
            "hilton2024",
            "tokyo2025",
            "london2025",
            "nyc2025",
            "starbucks2025",
        ];
        let locations: &'static [&'static str] = &[
            "Jakarta",
            "Bandung",
            "Surabaya",
            "Bali",
            "Medan",
            "Yogyakarta",
            "Makassar",
            "New York",
            "London",
            "Tokyo",
            "Berlin",
            "Paris",
            "Moscow",
            "Rio de Janeiro",
            "Sydney",
            "Toronto",
            "Dubai",
        ];
        let dobs: &'static [&'static str] = &[
            "01/01/2000",
            "12/12/2001",
            "05/05/2002",
            "23/07/2003",
            "14/02/2004",
            "30/08/2005",
            "17/11/2006",
            "04/07/1999",
            "31/12/1998",
            "15/03/1997",
            "22/11/1995",
            "09/09/1994",
            "28/02/1993",
            "10/10/1992",
        ];
        let cc_numbers: &'static [&'static str] = &[
            "1234-5678-9012-3456",
            "9876-5432-1098-7654",
            "1111-2222-3333-4444",
            "5555-6666-7777-8888",
            "9999-0000-1111-2222",
            "3333-4444-5555-6666",
            "7777-8888-9999-0000",
            "4000-1234-5678-9010",
            "5100-2345-6789-0123",
            "6011-1111-1111-1117",
            "3528-0000-0000-0000",
            "2222-4000-7000-0005",
        ];

        let container = Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("# **{target_name}'s** Hacked Data\n\n🔓 **Mission Complete!** Successfully infiltrated all systems."),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("**🔐 Device Password:**\n`{}`", pick(device_passwords, seed)),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("**🆔 System ID:**\n`{}`", pick(system_ids, seed / 2)),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("**📶 WiFi Access:**\n**Network:** {}\n**Password:** `{}`", pick(wifi_names, seed / 3), pick(wifi_pw, seed / 4)),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("**📍 Location:**\n{}", pick(locations, seed / 5)),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("**📋 DOB:**\n{}", pick(dobs, seed / 6)),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("**💳 Credit Card:**\n`{}`", pick(cc_numbers, seed / 7)),
                }),
                Component::Separator(Separator {
                    id: None,
                    divider: None,
                    spacing: Some(SeparatorSpacingSize::Large),
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: "-# This is completely fake and for entertainment purposes only".to_string(),
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
                        flags: Some(
                            twilight_model::channel::message::MessageFlags::IS_COMPONENTS_V2,
                        ),
                        ..Default::default()
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
