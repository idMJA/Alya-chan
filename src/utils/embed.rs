use chrono::Utc;
use twilight_model::channel::message::embed::Embed;
use twilight_model::util::Timestamp;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

const COLOR: u32 = 0xf48120;

pub fn make(title: &str, desc: &str, footer: Option<&str>) -> Embed {
    let env = std::env::var("NODE_ENV").unwrap_or_else(|_| "production".to_string());
    let env_tag = if env == "production" {
        "".to_string()
    } else {
        format!(" [{}]", env)
    };
    let full_title = format!("DNS over Discord{}: {}", env_tag, title);

    let mut embed = EmbedBuilder::new()
        .title(full_title)
        .description(desc)
        .color(COLOR)
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap());

    if let Some(f) = footer {
        embed = embed.footer(EmbedFooterBuilder::new(f));
    }

    embed.build()
}
