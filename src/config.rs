use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use url::Url;

/// Bot configuration loaded from `config.toml` (required)
#[derive(Debug, Clone)]
pub struct Config {
    pub color: ColorConfig,
    pub info: InfoConfig,
    pub emoji: EmojiConfig,
}

#[derive(Debug, Clone)]
pub struct ColorConfig {
    pub primary: u32,
    pub no: u32,
}

#[derive(Debug, Clone)]
pub struct InfoConfig {
    pub banner: String,
    pub invite_link: Option<String>,
    pub support_server: Option<String>,
    pub vote_link: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EmojiConfig {
    pub yes: String,
    pub no: String,
    pub link: String,
    pub party: String,
    pub artist: String,
    pub clock: String,
    pub user: String,
    pub info: String,
    pub music: String,
    pub warn: String,
    pub home: String,
    pub globe: String,
    pub slash: String,
    pub ping: String,
    pub question: String,
    pub pencil: String,
    pub think: String,
    pub heart: String,
    pub folder: String,
    pub play: String,
    pub pause: String,
    pub stop: String,
    pub skip: String,
    pub previous: String,
    pub rewind: String,
    pub forward: String,
    pub looping: String,
    pub shuffle: String,
    pub vol_up: String,
    pub vol_down: String,
    pub list: String,
    pub trash: String,
    pub node_on: String,
    pub node_off: String,
}

#[derive(Deserialize)]
struct RawConfig {
    color: RawColor,
    info: RawInfo,
    emoji: Option<RawEmoji>,
}

#[derive(Deserialize)]
struct RawColor {
    primary: toml::Value,
    no: Option<toml::Value>,
}

#[derive(Deserialize)]
struct RawInfo {
    banner: String,
    invite_link: Option<String>,
    support_server: Option<String>,
    vote_link: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawEmoji {
    yes: Option<String>,
    no: Option<String>,
    link: Option<String>,
    party: Option<String>,
    artist: Option<String>,
    clock: Option<String>,
    user: Option<String>,
    info: Option<String>,
    music: Option<String>,
    warn: Option<String>,
    home: Option<String>,
    globe: Option<String>,
    slash: Option<String>,
    ping: Option<String>,
    question: Option<String>,
    pencil: Option<String>,
    think: Option<String>,
    heart: Option<String>,
    folder: Option<String>,
    play: Option<String>,
    pause: Option<String>,
    stop: Option<String>,
    skip: Option<String>,
    previous: Option<String>,
    rewind: Option<String>,
    forward: Option<String>,
    r#loop: Option<String>,
    shuffle: Option<String>,
    vol_up: Option<String>,
    vol_down: Option<String>,
    list: Option<String>,
    trash: Option<String>,
    node_on: Option<String>,
    node_off: Option<String>,
}

impl Config {
    /// Load configuration from path. `config.toml` is required and must contain the required keys.
    pub fn load_from_path(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Config file not found or unreadable: {}", path))?;

        let raw: RawConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML in config file: {}", path))?;

        // Parse colors
        let primary = match parse_color_value(&raw.color.primary) {
            Some(v) => v,
            None => anyhow::bail!("Invalid or missing color.primary in config.toml"),
        };

        let no = match &raw.color.no {
            Some(v) => parse_color_value(v)
                .ok_or_else(|| anyhow::anyhow!("Invalid color.no in config.toml"))?,
            None => anyhow::bail!("Missing required key: color.no in config.toml"),
        };

        // Validate banner
        let banner = {
            let b = raw.info.banner;
            let parsed = Url::parse(&b)
                .with_context(|| format!("Invalid URL for config.info.banner: {}", b))?;
            let scheme = parsed.scheme();
            if scheme != "http" && scheme != "https" {
                anyhow::bail!("Unsupported URL scheme for config.info.banner: {}", b);
            }
            b
        };

        // Start building emoji config from raw (may be missing keys)
        let mut emoji = EmojiConfig::default();
        if let Some(r) = raw.emoji {
            if let Some(s) = r.yes {
                emoji.yes = s
            }
            if let Some(s) = r.no {
                emoji.no = s
            }
            if let Some(s) = r.link {
                emoji.link = s
            }
            if let Some(s) = r.party {
                emoji.party = s
            }
            if let Some(s) = r.artist {
                emoji.artist = s
            }
            if let Some(s) = r.clock {
                emoji.clock = s
            }
            if let Some(s) = r.user {
                emoji.user = s
            }
            if let Some(s) = r.info {
                emoji.info = s
            }
            if let Some(s) = r.music {
                emoji.music = s
            }
            if let Some(s) = r.warn {
                emoji.warn = s
            }
            if let Some(s) = r.home {
                emoji.home = s
            }
            if let Some(s) = r.globe {
                emoji.globe = s
            }
            if let Some(s) = r.slash {
                emoji.slash = s
            }
            if let Some(s) = r.ping {
                emoji.ping = s
            }
            if let Some(s) = r.question {
                emoji.question = s
            }
            if let Some(s) = r.pencil {
                emoji.pencil = s
            }
            if let Some(s) = r.think {
                emoji.think = s
            }
            if let Some(s) = r.heart {
                emoji.heart = s
            }
            if let Some(s) = r.folder {
                emoji.folder = s
            }
            if let Some(s) = r.play {
                emoji.play = s
            }
            if let Some(s) = r.pause {
                emoji.pause = s
            }
            if let Some(s) = r.stop {
                emoji.stop = s
            }
            if let Some(s) = r.skip {
                emoji.skip = s
            }
            if let Some(s) = r.previous {
                emoji.previous = s
            }
            if let Some(s) = r.rewind {
                emoji.rewind = s
            }
            if let Some(s) = r.forward {
                emoji.forward = s
            }
            if let Some(s) = r.r#loop {
                emoji.looping = s
            }
            if let Some(s) = r.shuffle {
                emoji.shuffle = s
            }
            if let Some(s) = r.vol_up {
                emoji.vol_up = s
            }
            if let Some(s) = r.vol_down {
                emoji.vol_down = s
            }
            if let Some(s) = r.list {
                emoji.list = s
            }
            if let Some(s) = r.trash {
                emoji.trash = s
            }
            if let Some(s) = r.node_on {
                emoji.node_on = s
            }
            if let Some(s) = r.node_off {
                emoji.node_off = s
            }
        }

        Ok(Self {
            color: ColorConfig { primary, no },
            info: InfoConfig {
                banner,
                invite_link: raw.info.invite_link,
                support_server: raw.info.support_server,
                vote_link: raw.info.vote_link,
            },
            emoji,
        })
    }
}

// call emoji overrides at the end of load
impl Config {
    pub fn load_with_overrides(path: &str) -> Result<Self> {
        let mut cfg = Self::load_from_path(path)?;
        load_emoji_overrides(&mut cfg);
        // Validate that all emoji fields are present after applying overrides
        let mut missing = Vec::new();

        if cfg.emoji.yes.is_empty() {
            missing.push("yes");
        }
        if cfg.emoji.no.is_empty() {
            missing.push("no");
        }
        if cfg.emoji.link.is_empty() {
            missing.push("link");
        }
        if cfg.emoji.party.is_empty() {
            missing.push("party");
        }
        if cfg.emoji.artist.is_empty() {
            missing.push("artist");
        }
        if cfg.emoji.clock.is_empty() {
            missing.push("clock");
        }
        if cfg.emoji.user.is_empty() {
            missing.push("user");
        }
        if cfg.emoji.info.is_empty() {
            missing.push("info");
        }
        if cfg.emoji.music.is_empty() {
            missing.push("music");
        }
        if cfg.emoji.warn.is_empty() {
            missing.push("warn");
        }
        if cfg.emoji.home.is_empty() {
            missing.push("home");
        }
        if cfg.emoji.globe.is_empty() {
            missing.push("globe");
        }
        if cfg.emoji.slash.is_empty() {
            missing.push("slash");
        }
        if cfg.emoji.ping.is_empty() {
            missing.push("ping");
        }
        if cfg.emoji.question.is_empty() {
            missing.push("question");
        }
        if cfg.emoji.pencil.is_empty() {
            missing.push("pencil");
        }
        if cfg.emoji.think.is_empty() {
            missing.push("think");
        }
        if cfg.emoji.heart.is_empty() {
            missing.push("heart");
        }
        if cfg.emoji.folder.is_empty() {
            missing.push("folder");
        }
        if cfg.emoji.play.is_empty() {
            missing.push("play");
        }
        if cfg.emoji.pause.is_empty() {
            missing.push("pause");
        }
        if cfg.emoji.stop.is_empty() {
            missing.push("stop");
        }
        if cfg.emoji.skip.is_empty() {
            missing.push("skip");
        }
        if cfg.emoji.previous.is_empty() {
            missing.push("previous");
        }
        if cfg.emoji.rewind.is_empty() {
            missing.push("rewind");
        }
        if cfg.emoji.forward.is_empty() {
            missing.push("forward");
        }
        if cfg.emoji.looping.is_empty() {
            missing.push("loop");
        }
        if cfg.emoji.shuffle.is_empty() {
            missing.push("shuffle");
        }
        if cfg.emoji.vol_up.is_empty() {
            missing.push("vol_up");
        }
        if cfg.emoji.vol_down.is_empty() {
            missing.push("vol_down");
        }
        if cfg.emoji.list.is_empty() {
            missing.push("list");
        }
        if cfg.emoji.trash.is_empty() {
            missing.push("trash");
        }
        if cfg.emoji.node_on.is_empty() {
            missing.push("node_on");
        }
        if cfg.emoji.node_off.is_empty() {
            missing.push("node_off");
        }

        if !missing.is_empty() {
            anyhow::bail!(
                "Missing required emoji keys after applying overrides: {}",
                missing.join(", ")
            );
        }

        Ok(cfg)
    }
}

// Try to load emoji overrides from a separate emoji.toml if present
fn load_emoji_overrides(cfg: &mut Config) {
    if let Ok(content) = fs::read_to_string("./emoji.toml") {
        // Try to parse file into the same RawEmoji struct (expects [emoji] table)
        #[derive(Deserialize)]
        struct EmojiFile {
            emoji: RawEmoji,
        }

        if let Ok(wrapper) = toml::from_str::<EmojiFile>(&content) {
            let r = wrapper.emoji;
            if let Some(s) = r.yes {
                cfg.emoji.yes = s
            }
            if let Some(s) = r.no {
                cfg.emoji.no = s
            }
            if let Some(s) = r.link {
                cfg.emoji.link = s
            }
            if let Some(s) = r.party {
                cfg.emoji.party = s
            }
            if let Some(s) = r.artist {
                cfg.emoji.artist = s
            }
            if let Some(s) = r.clock {
                cfg.emoji.clock = s
            }
            if let Some(s) = r.user {
                cfg.emoji.user = s
            }
            if let Some(s) = r.info {
                cfg.emoji.info = s
            }
            if let Some(s) = r.music {
                cfg.emoji.music = s
            }
            if let Some(s) = r.warn {
                cfg.emoji.warn = s
            }
            if let Some(s) = r.home {
                cfg.emoji.home = s
            }
            if let Some(s) = r.globe {
                cfg.emoji.globe = s
            }
            if let Some(s) = r.slash {
                cfg.emoji.slash = s
            }
            if let Some(s) = r.ping {
                cfg.emoji.ping = s
            }
            if let Some(s) = r.question {
                cfg.emoji.question = s
            }
            if let Some(s) = r.pencil {
                cfg.emoji.pencil = s
            }
            if let Some(s) = r.think {
                cfg.emoji.think = s
            }
            if let Some(s) = r.heart {
                cfg.emoji.heart = s
            }
            if let Some(s) = r.folder {
                cfg.emoji.folder = s
            }
            if let Some(s) = r.play {
                cfg.emoji.play = s
            }
            if let Some(s) = r.pause {
                cfg.emoji.pause = s
            }
            if let Some(s) = r.stop {
                cfg.emoji.stop = s
            }
            if let Some(s) = r.skip {
                cfg.emoji.skip = s
            }
            if let Some(s) = r.previous {
                cfg.emoji.previous = s
            }
            if let Some(s) = r.rewind {
                cfg.emoji.rewind = s
            }
            if let Some(s) = r.forward {
                cfg.emoji.forward = s
            }
            if let Some(s) = r.r#loop {
                cfg.emoji.looping = s
            }
            if let Some(s) = r.shuffle {
                cfg.emoji.shuffle = s
            }
            if let Some(s) = r.vol_up {
                cfg.emoji.vol_up = s
            }
            if let Some(s) = r.vol_down {
                cfg.emoji.vol_down = s
            }
            if let Some(s) = r.list {
                cfg.emoji.list = s
            }
            if let Some(s) = r.trash {
                cfg.emoji.trash = s
            }
            if let Some(s) = r.node_on {
                cfg.emoji.node_on = s
            }
            if let Some(s) = r.node_off {
                cfg.emoji.node_off = s
            }
        }
    }
}

fn parse_color_value(v: &toml::Value) -> Option<u32> {
    if let Some(i) = v.as_integer() {
        Some(i as u32)
    } else if let Some(s) = v.as_str() {
        parse_color_str(s)
    } else {
        None
    }
}

fn parse_color_str(s: &str) -> Option<u32> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    let s = s.strip_prefix("0x").unwrap_or(s);
    u32::from_str_radix(s, 16).ok()
}
