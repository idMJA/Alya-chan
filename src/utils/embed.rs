use chrono::Utc;
use twilight_model::channel::message::embed::Embed;
use twilight_model::util::Timestamp;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

const COLOR: u32 = 0x00_f4_81_20;

pub fn make(title: &str, desc: &str, footer: Option<&str>) -> Embed {
    let full_title = format!("DNS over Discord: {title}");

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
