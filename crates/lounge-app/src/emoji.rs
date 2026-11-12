use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

pub struct Emoji {
    pub shortcode: &'static str,
    pub glyph: &'static str,
}

fn load() -> Vec<Emoji> {
    let map: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../assets/simplemap.json")).unwrap_or_default();
    map.into_iter()
        .map(|(shortcode, glyph)| Emoji {
            shortcode: Box::leak(shortcode.into_boxed_str()),
            glyph: Box::leak(glyph.into_boxed_str()),
        })
        .collect()
}

pub fn all() -> &'static [Emoji] {
    static EMOJI: OnceLock<Vec<Emoji>> = OnceLock::new();
    EMOJI.get_or_init(load)
}

pub fn find(term: &str) -> Option<&'static Emoji> {
    let term = term.trim_start_matches(':').to_lowercase();
    if term.is_empty() {
        return None;
    }
    all()
        .iter()
        .find(|emoji| emoji.shortcode == term)
        .or_else(|| {
            all()
                .iter()
                .filter(|emoji| emoji.shortcode.starts_with(&term))
                .min_by_key(|emoji| emoji.shortcode.len())
        })
        .or_else(|| {
            all()
                .iter()
                .filter(|emoji| emoji.shortcode.contains(&term))
                .min_by_key(|emoji| emoji.shortcode.len())
        })
}

pub fn filter(term: &str, limit: usize) -> Vec<&'static Emoji> {
    let term = term.trim_start_matches(':').to_lowercase();
    all()
        .iter()
        .filter(|emoji| term.is_empty() || emoji.shortcode.contains(&term))
        .take(limit)
        .collect()
}

pub fn is_emoji_only(text: &str) -> bool {
    static CHARS: OnceLock<HashSet<char>> = OnceLock::new();
    let chars = CHARS.get_or_init(|| all().iter().flat_map(|emoji| emoji.glyph.chars()).collect());
    let mut count = 0;
    for character in text.chars() {
        if character.is_whitespace() {
            continue;
        }
        if character.is_ascii() || !chars.contains(&character) {
            return false;
        }
        count += 1;
    }
    count > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_emoji_only_messages() {
        assert!(is_emoji_only("😀"));
        assert!(is_emoji_only("😀 😎"));
        assert!(!is_emoji_only("hello 😀"));
        assert!(!is_emoji_only("123"));
        assert!(!is_emoji_only(""));
    }
}
