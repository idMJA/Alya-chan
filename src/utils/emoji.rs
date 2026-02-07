use twilight_model::channel::message::EmojiReactionType;
use twilight_model::id::Id;

/// Parse an emoji string from config (e.g. "<:yes:123456>") into a `Button` emoji struct.
/// Falls back to a Unicode emoji by setting `name` only.
pub fn parse_component_emoji(s: &str) -> Option<EmojiReactionType> {
    let s = s.trim();
    if s.starts_with('<') && s.ends_with('>') {
        let inner = &s[1..s.len() - 1];
        let parts: Vec<&str> = inner.split(':').collect();
        if parts.len() == 3 {
            let animated = parts[0] == "a";
            let name = parts[1];
            if let Ok(id_num) = parts[2].parse::<u64>() {
                return Some(EmojiReactionType::Custom {
                    animated,
                    id: Id::new(id_num),
                    name: Some(name.to_string()),
                });
            }
        }
    }

    if !s.is_empty() {
        return Some(EmojiReactionType::Unicode {
            name: s.to_string(),
        });
    }

    None
}
