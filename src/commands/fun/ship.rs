use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use crate::utils::{calc_love, love_msg, ship_name, ShipCanvas};
use async_trait::async_trait;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::http::attachment::Attachment;
use twilight_util::builder::command::{CommandBuilder, UserBuilder};

pub struct ShipCommand;

#[async_trait]
impl SlashCommand for ShipCommand {
    fn name(&self) -> &str {
        "ship"
    }

    fn description(&self) -> &str {
        "Shows the probability of two users being lovers!"
    }

    fn build(&self) -> Command {
        CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput)
            .option(UserBuilder::new("user1", "First user to ship").required(true))
            .option(UserBuilder::new("user2", "Second user to ship").required(true))
            .build()
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &twilight_model::http::interaction::InteractionResponse {
                    kind: twilight_model::http::interaction::InteractionResponseType::DeferredChannelMessageWithSource,
                    data: None,
                },
            )
            .await?;

        let mut uid1 = None;
        let mut uid2 = None;
        for opt in &ctx.data.options {
            if opt.name == "user1" {
                if let CommandOptionValue::User(id) = opt.value {
                    uid1 = Some(id);
                }
            }
            if opt.name == "user2" {
                if let CommandOptionValue::User(id) = opt.value {
                    uid2 = Some(id);
                }
            }
        }

        let uid1 = uid1.ok_or("user1 missing")?;
        let uid2 = uid2.ok_or("user2 missing")?;

        let u1 = ctx.bot.http.user(uid1).await?.model().await?;
        let u2 = ctx.bot.http.user(uid2).await?.model().await?;

        let pct = calc_love(&uid1.to_string(), &uid2.to_string());
        let name = ship_name(&u1.name, &u2.name);
        let (_emoji, msg) = love_msg(pct);

        let av1 = get_avatar(&u1.avatar, uid1.get()).await;
        let av2 = get_avatar(&u2.avatar, uid2.get()).await;

        let canvas = ShipCanvas::new().load_font("src/utils/fonts/norwester.otf");
        let bg_bytes = ShipCanvas::load_bg_bytes();
        let img = canvas.generate(&u1.name, &u2.name, av1, av2, pct, bg_bytes)?;

        let att = Attachment::from_bytes("ship.png".to_string(), img, 1);
        let content = format!(
            "## **{}** Ship Results\n\n👥 **{}** ❤️ **{}**\n\n### Love Percentage: **{}%**\n\n{}\n\n💡 **Fun Fact:** Ship names are created by combining parts of both usernames!",
            name, u1.name, u2.name, pct, msg
        );

        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_followup(&ctx.token)
            .content(&content)
            .attachments(&[att])
            .await?;

        Ok(())
    }
}

async fn get_avatar(hash: &Option<twilight_model::util::ImageHash>, id: u64) -> Option<Vec<u8>> {
    if let Some(h) = hash {
        let url = format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size=256",
            id, h
        );
        if let Ok(r) = reqwest::get(&url).await {
            if let Ok(b) = r.bytes().await {
                return Some(b.to_vec());
            }
        }
    }

    let url = format!(
        "https://cdn.discordapp.com/embed/avatars/{}.png",
        (id >> 22) % 6
    );
    reqwest::get(&url)
        .await
        .ok()?
        .bytes()
        .await
        .ok()
        .map(|b| b.to_vec())
}
