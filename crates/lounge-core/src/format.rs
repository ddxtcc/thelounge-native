use std::ops::Range;

const BOLD: char = '\x02';
const COLOR: char = '\x03';
const HEX_COLOR: char = '\x04';
const RESET: char = '\x0f';
const REVERSE: char = '\x16';
const ITALIC: char = '\x1d';
const STRIKETHROUGH: char = '\x1e';
const UNDERLINE: char = '\x1f';
const MONOSPACE: char = '\x11';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IrcColor {
    #[default]
    Default,
    Index(u8),
    Rgb(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span {
    pub text: String,
    pub fg: IrcColor,
    pub bg: IrcColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub monospace: bool,
}

impl Span {
    pub fn is_plain(&self) -> bool {
        self.fg == IrcColor::Default
            && self.bg == IrcColor::Default
            && !self.bold
            && !self.italic
            && !self.underline
            && !self.strikethrough
            && !self.monospace
    }

    fn same_style(&self, other: &Style) -> bool {
        self.fg == other.fg
            && self.bg == other.bg
            && self.bold == other.bold
            && self.italic == other.italic
            && self.underline == other.underline
            && self.strikethrough == other.strikethrough
            && self.monospace == other.monospace
    }
}

#[derive(Default, Clone)]
struct Style {
    fg: IrcColor,
    bg: IrcColor,
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    monospace: bool,
}

impl Style {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn span(&self, text: String) -> Span {
        Span {
            text,
            fg: self.fg,
            bg: self.bg,
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            strikethrough: self.strikethrough,
            monospace: self.monospace,
        }
    }
}

fn parse_color(text: &str, at: usize) -> Option<(IrcColor, IrcColor, usize)> {
    let rest = &text[at..];
    let mut digits = 0;
    let mut comma = None;
    for (i, ch) in rest.char_indices() {
        if ch.is_ascii_digit() && i < 2 {
            digits += 1;
        } else if ch == ',' && digits > 0 && comma.is_none() {
            comma = Some(i);
        } else {
            break;
        }
    }
    if digits == 0 {
        return Some((IrcColor::Default, IrcColor::Default, 0));
    }

    let fg = rest[..digits].parse::<u8>().ok()?;
    let consumed = match comma {
        Some(comma_pos) if comma_pos + 1 < rest.len() => {
            let bg_start = comma_pos + 1;
            let bg_digits = rest[bg_start..]
                .chars()
                .take(2)
                .take_while(|c| c.is_ascii_digit())
                .count();
            if bg_digits > 0 {
                let bg = rest[bg_start..bg_start + bg_digits].parse::<u8>().ok()?;
                (
                    IrcColor::Index(fg),
                    IrcColor::Index(bg),
                    bg_start + bg_digits,
                )
            } else {
                (IrcColor::Index(fg), IrcColor::Default, digits)
            }
        }
        _ => (IrcColor::Index(fg), IrcColor::Default, digits),
    };
    Some(consumed)
}

fn parse_hex_color(text: &str, at: usize) -> Option<(IrcColor, IrcColor, usize)> {
    let rest = &text[at..];
    let mut fg_len = 0;
    for (i, ch) in rest.char_indices() {
        if ch.is_ascii_hexdigit() && i < 6 {
            fg_len += 1;
        } else {
            break;
        }
    }
    if fg_len != 6 {
        return None;
    }
    let fg = u32::from_str_radix(&rest[..6], 16).ok()?;
    let mut consumed = 6;
    let mut bg = IrcColor::Default;
    if rest[6..].starts_with(',') {
        let after = &rest[7..];
        let mut len = 0;
        for (i, ch) in after.char_indices() {
            if ch.is_ascii_hexdigit() && i < 6 {
                len += 1;
            } else {
                break;
            }
        }
        if len == 6 {
            bg = IrcColor::Rgb(u32::from_str_radix(&after[..6], 16).ok()?);
            consumed = 7 + 6;
        }
    }
    Some((IrcColor::Rgb(fg), bg, consumed))
}

fn is_control(ch: char) -> bool {
    matches!(
        ch,
        '\u{0000}'..='\u{0009}' | '\u{000B}'..='\u{001F}'
    )
}

pub fn parse(text: &str) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let mut style = Style::default();
    let mut buffer = String::new();
    let mut chars = text.char_indices().peekable();

    fn flush(spans: &mut Vec<Span>, style: &Style, buffer: &mut String) {
        if buffer.is_empty() {
            return;
        }
        if let Some(last) = spans.last_mut()
            && last.same_style(style)
        {
            last.text.push_str(buffer);
            buffer.clear();
            return;
        }
        spans.push(style.span(std::mem::take(buffer)));
    }

    while let Some((i, ch)) = chars.next() {
        match ch {
            RESET => {
                flush(&mut spans, &style, &mut buffer);
                style.reset();
            }
            BOLD => {
                flush(&mut spans, &style, &mut buffer);
                style.bold = !style.bold;
            }
            ITALIC => {
                flush(&mut spans, &style, &mut buffer);
                style.italic = !style.italic;
            }
            UNDERLINE => {
                flush(&mut spans, &style, &mut buffer);
                style.underline = !style.underline;
            }
            STRIKETHROUGH => {
                flush(&mut spans, &style, &mut buffer);
                style.strikethrough = !style.strikethrough;
            }
            MONOSPACE => {
                flush(&mut spans, &style, &mut buffer);
                style.monospace = !style.monospace;
            }
            REVERSE => {
                flush(&mut spans, &style, &mut buffer);
                std::mem::swap(&mut style.fg, &mut style.bg);
            }
            COLOR => {
                flush(&mut spans, &style, &mut buffer);
                if let Some((fg, bg, consumed)) = parse_color(text, i + 1) {
                    style.fg = fg;
                    style.bg = bg;
                    for _ in 0..consumed {
                        chars.next();
                    }
                }
            }
            HEX_COLOR => {
                flush(&mut spans, &style, &mut buffer);
                if let Some((fg, bg, consumed)) = parse_hex_color(text, i + 1) {
                    style.fg = fg;
                    style.bg = bg;
                    for _ in 0..consumed {
                        chars.next();
                    }
                }
            }
            _ if is_control(ch) => {
                if ch == '\n' || ch == '\t' {
                    buffer.push(ch);
                }
            }
            _ => buffer.push(ch),
        }
    }

    flush(&mut spans, &style, &mut buffer);
    spans
}

pub fn clean(text: &str) -> String {
    parse(text).into_iter().map(|span| span.text).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedLink {
    pub range: Range<usize>,
    pub url: String,
}

fn is_url_start(bytes: &[u8], i: usize) -> Option<usize> {
    let rest = &bytes[i..];
    for scheme in [b"https://".as_slice(), b"http://".as_slice()] {
        if rest.len() >= scheme.len() && rest[..scheme.len()].eq_ignore_ascii_case(scheme) {
            return Some(scheme.len());
        }
    }
    None
}

fn trim_trailing(url: &str) -> &str {
    let mut end = url.len();
    while let Some(last) = url[..end].chars().next_back() {
        if !matches!(
            last,
            '.' | ',' | ';' | ':' | '!' | '?' | '\'' | '"' | ')' | ']' | '}'
        ) {
            break;
        }
        // Keep balanced closing brackets: `(https://example.com)` should drop
        if matches!(last, ')' | ']' | '}') {
            let (open, close) = match last {
                ')' => ('(', ')'),
                ']' => ('[', ']'),
                _ => ('{', '}'),
            };
            if url[..end].matches(open).count() >= url[..end].matches(close).count() {
                break;
            }
        }
        end -= last.len_utf8();
    }
    &url[..end]
}

pub fn find_links(text: &str) -> Vec<DetectedLink> {
    let bytes = text.as_bytes();
    let mut links = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(_scheme_len) = is_url_start(bytes, i) {
            let mut end = i;
            while end < bytes.len() {
                let b = bytes[end];
                if b.is_ascii_whitespace() || b == b'<' || b == b'>' || b < 0x20 {
                    break;
                }
                end += 1;
            }
            let raw = &text[i..end];
            let url = trim_trailing(raw);
            if url.len() > 8 {
                links.push(DetectedLink {
                    range: i..i + url.len(),
                    url: url.to_string(),
                });
            }
            i = end.max(i + 1);
        } else {
            i += text[i..].chars().next().map(char::len_utf8).unwrap_or(1);
        }
    }
    links
}

pub fn find_nicks(text: &str, nicks: &[String]) -> Vec<(Range<usize>, String)> {
    let links = find_links(text);
    let mut found = Vec::new();
    for nick in nicks {
        if nick.is_empty() {
            continue;
        }
        let lower_text = text.to_lowercase();
        let lower_nick = nick.to_lowercase();
        let mut from = 0;
        while let Some(pos) = lower_text[from..].find(&lower_nick) {
            let start = from + pos;
            let end = start + lower_nick.len();
            let before_ok = text[..start]
                .chars()
                .next_back()
                .map(|c| !c.is_alphanumeric() && c != '_' && c != '-')
                .unwrap_or(true);
            let after_ok = text[end..]
                .chars()
                .next()
                .map(|c| !c.is_alphanumeric() && c != '_' && c != '-')
                .unwrap_or(true);
            let in_link = links
                .iter()
                .any(|l| l.range.start < end && start < l.range.end);
            if before_ok && after_ok && !in_link {
                found.push((start..end, nick.clone()));
            }
            from = end;
            if from >= lower_text.len() {
                break;
            }
        }
    }
    found.sort_by_key(|(range, _)| range.start);
    found
}

pub fn find_channels(text: &str) -> Vec<(Range<usize>, String)> {
    let links = find_links(text);
    let mut found = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#'
            && (i == 0
                || !text[..i]
                    .chars()
                    .next_back()
                    .map(|c| c.is_alphanumeric() || c == '_' || c == '-')
                    .unwrap_or(false))
        {
            let mut end = i + 1;
            while end < bytes.len() {
                let c = bytes[end] as char;
                if c.is_alphanumeric()
                    || matches!(
                        c,
                        '_' | '-' | '[' | ']' | '\\' | '`' | '^' | '{' | '}' | '|'
                    )
                {
                    end += 1;
                } else {
                    break;
                }
            }
            let in_link = links.iter().any(|l| l.range.start < end && i < l.range.end);
            if end > i + 1 && !in_link {
                found.push((i..end, text[i..end].to_string()));
            }
            i = end;
        } else {
            i += text[i..].chars().next().map(char::len_utf8).unwrap_or(1);
        }
    }
    found
}

pub const NICK_COLORS: [u32; 32] = [
    0x50a656, 0x107ead, 0xa86500, 0x008a3c, 0xe00096, 0xf0000c, 0x000094, 0x006441, 0x00566e,
    0xe6006b, 0x0d8766, 0x006b3b, 0x00857e, 0x00465b, 0xeb005a, 0xe62600, 0x0f8546, 0xe60067,
    0xeb002b, 0xeb003f, 0x007a56, 0x095092, 0x000bde, 0x008577, 0x00367d, 0x007e9e, 0x006119,
    0x007ea8, 0x3c8500, 0xe6007e, 0xc75300, 0xeb0400,
];

pub fn nick_color(nick: &str) -> u32 {
    let mut hash: u32 = 0;
    for ch in nick.chars() {
        hash = hash.wrapping_add(ch as u32);
    }
    NICK_COLORS[(hash as usize) % NICK_COLORS.len()]
}

pub const IRC_PALETTE: [u32; 16] = [
    0xffffff, 0x000000, 0x001f3f, 0x2ecc40, 0xff4136, 0x85144b, 0xb10dc9, 0xff851b, 0xffdc00,
    0x01ff70, 0x39cccc, 0x7fdbff, 0x0074d9, 0xf012be, 0xaaaaaa, 0xdddddd,
];

pub fn color_to_rgb(color: IrcColor) -> Option<u32> {
    match color {
        IrcColor::Default => None,
        IrcColor::Rgb(rgb) => Some(rgb),
        IrcColor::Index(index) => IRC_PALETTE.get(index as usize).copied(),
    }
}

pub fn format_time(
    time: Option<chrono::DateTime<chrono::Utc>>,
    seconds: bool,
    use_12h: bool,
) -> String {
    use chrono::Local;
    let Some(time) = time else {
        return String::new();
    };
    let time = time.with_timezone(&Local);
    let fmt = match (use_12h, seconds) {
        (false, false) => "%H:%M",
        (false, true) => "%H:%M:%S",
        (true, false) => "%I:%M %p",
        (true, true) => "%I:%M:%S %p",
    };
    time.format(fmt).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_formatting() {
        assert_eq!(clean("\x02bold\x02 and \x1ditalic\x1d"), "bold and italic");
        assert_eq!(clean("\x0304red\x03 normal"), "red normal");
        assert_eq!(clean("\x0304,02red bg\x03"), "red bg");
        assert_eq!(clean("\x0304"), "");
        assert_eq!(clean("\x0fplain"), "plain");
    }

    #[test]
    fn hex_colors() {
        let spans = parse("\x04ff0000red\x04");
        assert_eq!(spans[0].fg, IrcColor::Rgb(0xff0000));
        assert_eq!(spans[0].text, "red");
    }

    #[test]
    fn toggles_and_merges() {
        let spans = parse("\x02hel\x02lo");
        assert_eq!(spans.len(), 2);
        assert!(spans[0].bold);
        assert!(!spans[1].bold);
        assert_eq!(spans[0].text, "hel");
        assert_eq!(spans[1].text, "lo");

        let spans = parse("\x02a\x02b");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "a");
        assert!(spans[0].bold);
        assert_eq!(spans[1].text, "b");
        assert!(!spans[1].bold);
    }

    #[test]
    fn reverse_swaps_colors() {
        let spans = parse("\x0304,02\x16rev\x0f");
        assert_eq!(spans[0].fg, IrcColor::Index(2));
        assert_eq!(spans[0].bg, IrcColor::Index(4));
    }

    #[test]
    fn links() {
        let links = find_links("see https://example.com/a?b=1, and (http://x.y/z).");
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].url, "https://example.com/a?b=1");
        assert_eq!(links[1].url, "http://x.y/z");
        assert_eq!(
            &"see https://example.com/a?b=1, and (http://x.y/z)."[links[0].range.clone()],
            "https://example.com/a?b=1"
        );
        assert!(find_links("nothing here").is_empty());
    }

    #[test]
    fn nick_mentions_skip_words() {
        let found = find_nicks("alice: alice2 and Alice", &["alice".into()]);
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].0, 18..23);
    }

    #[test]
    fn channels() {
        let found = find_channels("join #rust and #rust-lang, email a#b");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].1, "#rust");
        assert_eq!(found[1].1, "#rust-lang");
    }
}
