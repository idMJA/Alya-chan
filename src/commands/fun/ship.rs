use crate::types::{BotResult, SlashCommand, SlashCommandContext};
use async_trait::async_trait;
use skia_safe::{surfaces, Color, Font, Paint, Rect};
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::message::component::{
    Component, Container, MediaGallery, MediaGalleryItem, Separator, SeparatorSpacingSize,
    TextDisplay, UnfurledMediaItem,
};
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_util::builder::command::{CommandBuilder, UserBuilder};

pub struct ShipCommand;

const CANVAS_WIDTH: i32 = 1280;
const CANVAS_HEIGHT: i32 = 720;

const AVATAR_SIZE: i32 = 97;
const AVATAR1_X: f32 = 619.4;
const AVATAR1_Y: f32 = 205.1;
const AVATAR2_X: f32 = 151.0;
const AVATAR2_Y: f32 = 433.7;

const BACKGROUND_URL: &str = "https://i.postimg.cc/655CqYWT/4-20250720-072352-0001.png";

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
            .option(UserBuilder::new("user", "The 1st user you want to ship!").required(true))
            .option(UserBuilder::new("member", "The 2nd user you want to ship!").required(true))
            .build()
    }

    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        let mut user_id = None;
        let mut member_id = None;

        for opt in &ctx.data.options {
            match opt.name.as_str() {
                "user" => {
                    if let CommandOptionValue::User(id) = opt.value {
                        user_id = Some(id);
                    }
                }
                "member" => {
                    if let CommandOptionValue::User(id) = opt.value {
                        member_id = Some(id);
                    }
                }
                _ => {}
            }
        }

        let (user_id, member_id) = match (user_id, member_id) {
            (Some(u), Some(m)) => (u, m),
            _ => {
                return send_error_response(ctx, "❌ **Error**\n\nPlease select two users.").await;
            }
        };

        let client = reqwest::Client::new();

        let user = match ctx.bot.http.user(user_id).await {
            Ok(resp) => match resp.model().await {
                Ok(user) => user,
                Err(_) => {
                    return send_error_response(ctx, "❌ **Error**\n\nFailed to fetch user data.")
                        .await;
                }
            },
            Err(_) => {
                return send_error_response(ctx, "❌ **Error**\n\nFailed to fetch user data.")
                    .await;
            }
        };

        let member = match ctx.bot.http.user(member_id).await {
            Ok(resp) => match resp.model().await {
                Ok(user) => user,
                Err(_) => {
                    return send_error_response(
                        ctx,
                        "❌ **Error**\n\nFailed to fetch member data.",
                    )
                    .await;
                }
            },
            Err(_) => {
                return send_error_response(ctx, "❌ **Error**\n\nFailed to fetch member data.")
                    .await;
            }
        };

        let requester_name = if let Some(author_id) = ctx.author_id {
            match ctx.bot.http.user(author_id).await {
                Ok(resp) => resp
                    .model()
                    .await
                    .map(|u| u.name)
                    .unwrap_or_else(|_| "someone".to_string()),
                Err(_) => "someone".to_string(),
            }
        } else {
            "someone".to_string()
        };

        let combined = format!("{}{}", user.id, member.id);
        let hash = combined
            .chars()
            .fold(0i32, |acc, ch| (acc << 5).wrapping_sub(acc) + ch as i32);
        let love_percentage = (hash.unsigned_abs() % 101) as u32;

        let ship_name = build_ship_name(&user.name, &member.name);
        let (love_message, emoji) = love_message_for(love_percentage);

        let avatar1_url = avatar_url(&user);
        let avatar2_url = avatar_url(&member);

        // Fetch images
        let avatar1_img = fetch_image(&client, &avatar1_url).await;
        let avatar2_img = fetch_image(&client, &avatar2_url).await;

        // Render canvas
        let png_bytes = match render_ship_canvas(
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
            &avatar1_img,
            &avatar2_img,
            love_percentage,
        ) {
            Ok(bytes) => bytes,
            Err(_) => {
                return send_error_response(ctx, "❌ **Error**\n\nFailed to render ship image.")
                    .await;
            }
        };

        let attachment = Attachment::from_bytes("ship.png".to_string(), png_bytes, 1);

        let container = Container {
            id: None,
            components: vec![
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!("# {} Love Calculator", emoji),
                }),
                Component::MediaGallery(MediaGallery {
                    id: None,
                    items: vec![MediaGalleryItem {
                        media: UnfurledMediaItem {
                            url: "attachment://ship.png".to_string(),
                            content_type: Some("image/png".to_string()),
                            height: None,
                            width: None,
                            proxy_url: None,
                        },
                        description: Some(format!("{} Ship Results", ship_name)),
                        spoiler: None,
                    }],
                }),
                Component::TextDisplay(TextDisplay {
                    id: None,
                    content: format!(
                        "## **{}** Ship Results\n\n👥 **{}** ❤️ **{}**\n\n### Love Percentage: **{}%**\n\n{}",
                        ship_name,
                        user.global_name.clone().unwrap_or_else(|| user.name.clone()),
                        member
                            .global_name
                            .clone()
                            .unwrap_or_else(|| member.name.clone()),
                        love_percentage,
                        love_message
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
                        "💡 **Fun Fact:** Ship names are created by combining parts of both usernames!\n\n-# Requested by {}",
                        requester_name
                    ),
                }),
            ],
            accent_color: Some(Some(ctx.bot.config.color.primary)),
            spoiler: None,
        };

        // Direct response with Components V2 and attachment
        ctx.bot
            .http
            .interaction(ctx.application_id.cast())
            .create_response(
                ctx.interaction_id.cast(),
                &ctx.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        attachments: Some(vec![attachment]),
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

async fn send_error_response(ctx: &SlashCommandContext, message: &str) -> BotResult<()> {
    let container = Container {
        id: None,
        components: vec![Component::TextDisplay(TextDisplay {
            id: None,
            content: message.to_string(),
        })],
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

fn render_ship_canvas(
    width: i32,
    height: i32,
    avatar1: &Option<Vec<u8>>,
    avatar2: &Option<Vec<u8>>,
    love_percentage: u32,
) -> Result<Vec<u8>, String> {
    // Create Skia surface
    let mut surface = surfaces::raster(
        &skia_safe::ImageInfo::new(
            skia_safe::ISize::new(width, height),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Unpremul,
            None,
        ),
        None,
        None,
    )
    .ok_or("Failed to create surface".to_string())?;

    let canvas = surface.canvas();

    // Draw gradient background
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgb(255, 107, 138)); // #ff6b8a
    canvas.draw_rect(
        Rect::from_xywh(0.0, 0.0, width as f32, height as f32),
        &paint,
    );

    // Overlay gradient
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgb(217, 70, 239)); // #d946ef
    paint.set_alpha(128);
    canvas.draw_rect(
        Rect::from_xywh(0.0, 0.0, width as f32, height as f32),
        &paint,
    );

    // Avatar constants
    const AVATAR_SIZE: f32 = 97.2;
    const AVATAR1_X: f32 = 619.4;
    const AVATAR1_Y: f32 = 205.1;
    const AVATAR2_X: f32 = 151.0;
    const AVATAR2_Y: f32 = 433.7;

    // Draw circular avatar 1
    if let Some(avatar_data) = avatar1 {
        if let Ok(img) = image::load_from_memory(avatar_data) {
            let resized = img.resize(
                AVATAR_SIZE as u32,
                AVATAR_SIZE as u32,
                image::imageops::FilterType::Lanczos3,
            );
            let rgba = resized.to_rgba8();

            // Create circular mask by drawing to temp surface
            let cx = AVATAR1_X + AVATAR_SIZE / 2.0;
            let cy = AVATAR1_Y + AVATAR_SIZE / 2.0;
            let radius = AVATAR_SIZE / 2.0;

            // Draw avatar pixels with circular clipping
            for (x, y, pixel) in rgba.enumerate_pixels() {
                let px = AVATAR1_X + x as f32;
                let py = AVATAR1_Y + y as f32;

                let dx = px - cx;
                let dy = py - cy;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance <= radius {
                    let color = Color::from_argb(pixel[3], pixel[0], pixel[1], pixel[2]);
                    let mut paint = Paint::default();
                    paint.set_color(color);
                    canvas.draw_circle((px, py), 0.5, &paint);
                }
            }
        }
    }

    // Draw circular avatar 2
    if let Some(avatar_data) = avatar2 {
        if let Ok(img) = image::load_from_memory(avatar_data) {
            let resized = img.resize(
                AVATAR_SIZE as u32,
                AVATAR_SIZE as u32,
                image::imageops::FilterType::Lanczos3,
            );
            let rgba = resized.to_rgba8();

            let cx = AVATAR2_X + AVATAR_SIZE / 2.0;
            let cy = AVATAR2_Y + AVATAR_SIZE / 2.0;
            let radius = AVATAR_SIZE / 2.0;

            for (x, y, pixel) in rgba.enumerate_pixels() {
                let px = AVATAR2_X + x as f32;
                let py = AVATAR2_Y + y as f32;

                let dx = px - cx;
                let dy = py - cy;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance <= radius {
                    let color = Color::from_argb(pixel[3], pixel[0], pixel[1], pixel[2]);
                    let mut paint = Paint::default();
                    paint.set_color(color);
                    canvas.draw_circle((px, py), 0.5, &paint);
                }
            }
        }
    }

    // Draw love percentage text (centered at 455, 363.5)
    let mut paint = Paint::default();
    paint.set_color(Color::WHITE);
    paint.set_anti_alias(true);

    let font = Font::default();
    let percentage_text = format!("{}%", love_percentage);

    // Draw text with shadow for better visibility
    let mut shadow_paint = Paint::default();
    shadow_paint.set_color(Color::from_argb(128, 0, 0, 0));
    canvas.draw_str(&percentage_text, (457.0, 365.0), &font, &shadow_paint);

    canvas.draw_str(&percentage_text, (455.0, 363.0), &font, &paint);

    // Encode to PNG
    let image = surface.image_snapshot();
    let data = image
        .encode_to_data(skia_safe::EncodedImageFormat::PNG)
        .ok_or("Failed to encode PNG".to_string())?;
    Ok(data.as_ref().to_vec())
}

fn avatar_url(user: &twilight_model::user::User) -> String {
    if let Some(hash) = user.avatar.as_ref() {
        format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size=256",
            user.id, hash
        )
    } else {
        let index = u16::from(user.discriminator) % 5;
        format!("https://cdn.discordapp.com/embed/avatars/{}.png", index)
    }
}

async fn fetch_image(client: &reqwest::Client, url: &str) -> Option<Vec<u8>> {
    let response = client.get(url).send().await.ok()?;
    response.bytes().await.ok().map(|b| b.to_vec())
}

fn build_ship_name(user: &str, member: &str) -> String {
    let user_chars: Vec<char> = user.chars().collect();
    let member_chars: Vec<char> = member.chars().collect();

    let user_half = (user_chars.len() + 1) / 2;
    let member_half = member_chars.len() / 2;

    let name1: String = user_chars.into_iter().take(user_half).collect();
    let name2: String = member_chars.into_iter().skip(member_half).collect();

    format!("{}{}", name1, name2)
}

fn love_message_for(percentage: u32) -> (String, &'static str) {
    if percentage >= 90 {
        (
            "Absolutely perfect! You two are soulmates! The universe conspired to bring you together! 💫"
                .to_string(),
            "💖",
        )
    } else if percentage >= 80 {
        (
            "Perfect match! You two are meant to be together! There's undeniable chemistry here! ✨"
                .to_string(),
            "💖",
        )
    } else if percentage >= 70 {
        (
            "Excellent compatibility! This relationship has all the right ingredients for success! 🌟"
                .to_string(),
            "💕",
        )
    } else if percentage >= 60 {
        (
            "Great compatibility! There's definitely something special brewing between you two! 💫"
                .to_string(),
            "💕",
        )
    } else if percentage >= 50 {
        (
            "Good potential! With some effort and understanding, this could blossom into something beautiful! 🌸"
                .to_string(),
            "💓",
        )
    } else if percentage >= 40 {
        (
            "Moderate compatibility. There are some sparks, but it might take work to fan the flames! 🔥"
                .to_string(),
            "💓",
        )
    } else if percentage >= 30 {
        (
            "Some chemistry detected, but there might be some challenges to overcome! 💪"
                .to_string(),
            "💔",
        )
    } else if percentage >= 20 {
        (
            "Limited compatibility. Friendship might be a better foundation than romance! 🤝"
                .to_string(),
            "💔",
        )
    } else if percentage >= 10 {
        (
            "Very little romantic chemistry. You're probably better as friends! 👫".to_string(),
            "💙",
        )
    } else {
        (
            "No romantic spark detected! But hey, the best relationships often start as friendships! 💙"
                .to_string(),
            "💙",
        )
    }
}
