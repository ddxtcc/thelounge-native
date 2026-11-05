use std::collections::BTreeMap;
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
