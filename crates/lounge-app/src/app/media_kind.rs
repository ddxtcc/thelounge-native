#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MediaKind {
    Video,
    Audio,
}

pub(super) fn media_kind(url: &str) -> Option<MediaKind> {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase();
    let extension = path.rsplit('.').next()?;
    match extension {
        "mp4" | "m4v" | "webm" | "mkv" | "mov" => Some(MediaKind::Video),
        "mp3" | "m4a" | "aac" | "ogg" | "oga" | "opus" | "wav" | "flac" => Some(MediaKind::Audio),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_direct_media_urls() {
        assert!(matches!(
            media_kind("https://example.com/a.webm?x=1"),
            Some(MediaKind::Video)
        ));
        assert!(matches!(
            media_kind("https://example.com/a.mp3"),
            Some(MediaKind::Audio)
        ));
        assert_eq!(media_kind("https://example.com/a.png"), None);
        assert_eq!(media_kind("https://example.com/a"), None);
    }
}
