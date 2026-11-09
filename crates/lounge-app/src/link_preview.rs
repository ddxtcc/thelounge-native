use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;

use gpui::{AppContext, Context};
use lounge_core::proto::LinkPreview;
use scraper::{Html, Selector};

use crate::app::LoungeApp;

const LIMIT: usize = 32;

#[derive(Clone)]
pub(crate) enum PreviewState {
    Loading,
    Ready(Box<LinkPreview>),
    Failed,
}

pub fn is_previewable_link(url: &str) -> bool {
    if !is_safe_remote_url(url, false) {
        return false;
    }
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    let path = parsed.path().to_ascii_lowercase();
    ![
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".avif", ".bmp", ".svg", ".mp3", ".mp4", ".m4a",
        ".webm", ".mkv", ".mov", ".ogg", ".oga", ".opus", ".wav", ".flac",
    ]
    .iter()
    .any(|extension| path.ends_with(extension))
}

pub(crate) fn is_safe_remote_url(url: &str, resolve_dns: bool) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.username() != ""
        || parsed.password().is_some()
    {
        return false;
    }
    let Some(host) = parsed.host() else {
        return false;
    };
    let port = parsed.port_or_known_default().unwrap_or(443);
    match host {
        url::Host::Ipv4(ip) => is_global_ip(IpAddr::V4(ip)),
        url::Host::Ipv6(ip) => {
            if let Some(ip) = ip.to_ipv4_mapped() {
                is_global_ip(IpAddr::V4(ip))
            } else {
                is_global_ip(IpAddr::V6(ip))
            }
        }
        url::Host::Domain(domain) => {
            let domain = domain.trim_end_matches('.').to_ascii_lowercase();
            if domain == "localhost" || domain.ends_with(".localhost") {
                return false;
            }
            if !resolve_dns {
                return true;
            }
            let Ok(addresses) = (domain.as_str(), port).to_socket_addrs() else {
                return false;
            };
            let addresses: Vec<_> = addresses.collect();
            !addresses.is_empty() && addresses.iter().all(|address| is_global_ip(address.ip()))
        }
    }
}

fn is_global_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_multicast())
        }
        IpAddr::V6(ip) => {
            !(ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.is_multicast())
        }
    }
}

pub fn fetch(url: &str) -> Result<LinkPreview, String> {
    if !is_previewable_link(url) || !is_safe_remote_url(url, true) {
        return Err("not a previewable link".to_string());
    }
    let mut response = ureq::get(url)
        .config()
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(12)))
        .build()
        .header("Accept", "text/html,application/xhtml+xml")
        .header("User-Agent", "lounge-native")
        .call()
        .map_err(|error| error.to_string())?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !content_type.is_empty() && !content_type.contains("text/html") {
        return Err("not an HTML page".to_string());
    }
    let bytes = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_vec()
        .map_err(|error| error.to_string())?;
    let html = String::from_utf8_lossy(&bytes);
    parse(url, &html)
}

fn parse(url: &str, html: &str) -> Result<LinkPreview, String> {
    let document = Html::parse_document(html);
    let meta_selector = Selector::parse("meta").map_err(|error| error.to_string())?;
    let title_selector = Selector::parse("title").map_err(|error| error.to_string())?;
    let mut head = String::new();
    let mut body = String::new();
    let mut thumb = String::new();
    for meta in document.select(&meta_selector) {
        let key = meta
            .value()
            .attr("property")
            .or_else(|| meta.value().attr("name"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let content = meta.value().attr("content").unwrap_or_default().trim();
        if content.is_empty() {
            continue;
        }
        match key.as_str() {
            "og:title" | "twitter:title" if head.is_empty() => head = content.to_string(),
            "og:description" | "twitter:description" | "description" if body.is_empty() => {
                body = content.to_string()
            }
            "og:image" | "twitter:image" if thumb.is_empty() => thumb = content.to_string(),
            _ => {}
        }
    }
    if head.is_empty()
        && let Some(title) = document.select(&title_selector).next()
    {
        head = title.text().collect::<String>().trim().to_string();
    }
    if head.is_empty() && body.is_empty() && thumb.is_empty() {
        return Err("no preview metadata".to_string());
    }
    let base = url::Url::parse(url).map_err(|error| error.to_string())?;
    let thumb = if thumb.is_empty() {
        String::new()
    } else {
        base.join(&thumb)
            .map(|url| url.to_string())
            .unwrap_or(thumb)
    };
    let head = if head.is_empty() {
        "Untitled page".to_string()
    } else {
        truncate(&head, 100)
    };
    Ok(LinkPreview {
        kind: "link".to_string(),
        head,
        body: truncate(&body, 300),
        thumb,
        size: 0,
        link: url.to_string(),
        shown: None,
        error: None,
        message: None,
        media: None,
        media_type: None,
        max_size: None,
        thumb_actual_url: None,
    })
}

fn truncate(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

impl LoungeApp {
    pub(crate) fn local_link_preview(
        &self,
        url: &str,
        cx: &mut Context<Self>,
    ) -> Option<LinkPreview> {
        if !self.previews_enabled() {
            return None;
        }
        if let Some(state) = self.link_previews.borrow().get(url) {
            return match state {
                PreviewState::Ready(preview) => Some((**preview).clone()),
                PreviewState::Loading | PreviewState::Failed => None,
            };
        }
        if self.link_previews.borrow().len() >= LIMIT {
            self.link_previews.borrow_mut().clear();
        }
        self.link_previews
            .borrow_mut()
            .insert(url.to_string(), PreviewState::Loading);
        let target = url.to_string();
        let key = target.clone();
        let task = cx.background_spawn(async move { fetch(&target) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                let state = match result {
                    Ok(preview) => PreviewState::Ready(Box::new(preview)),
                    Err(_) => PreviewState::Failed,
                };
                this.link_previews.borrow_mut().insert(key, state);
                cx.notify();
            });
        })
        .detach();
        None
    }

    pub(crate) fn clear_link_previews(&mut self) {
        self.link_previews.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_private_remote_urls() {
        for url in [
            "http://127.0.0.1/",
            "http://10.0.0.1/",
            "http://169.254.169.254/",
            "http://[::1]/",
            "http://[::ffff:127.0.0.1]/",
            "http://localhost/",
            "http://localhost./",
            "file:///etc/passwd",
            "http://user:pass@example.com/",
        ] {
            assert!(!is_safe_remote_url(url, false), "{url}");
        }
        assert!(is_safe_remote_url("https://example.com/", false));
    }

    #[test]
    fn parses_open_graph_metadata() {
        let html = r#"
            <html><head>
            <title>Fallback</title>
            <meta property="og:title" content="Example title">
            <meta property="og:description" content="Example description">
            <meta property="og:image" content="/image.png">
            </head></html>
        "#;
        let preview = parse("https://example.com/page", html).unwrap();
        assert_eq!(preview.head, "Example title");
        assert_eq!(preview.body, "Example description");
        assert_eq!(preview.thumb, "https://example.com/image.png");
    }

    #[test]
    fn falls_back_to_title_and_rejects_empty_metadata() {
        let preview = parse("https://example.com", "<title>Example</title>").unwrap();
        assert_eq!(preview.head, "Example");
        assert!(parse("https://example.com", "<html></html>").is_err());
    }

    #[test]
    fn skips_direct_media_links() {
        assert!(is_previewable_link("https://example.com/page"));
        assert!(!is_previewable_link("https://example.com/image.png"));
        assert!(!is_previewable_link("https://example.com/video.mp4"));
        assert!(!is_previewable_link("file:///tmp/page"));
        assert!(!is_previewable_link("http://localhost/page"));
        assert!(!is_previewable_link("http://127.0.0.1/page"));
        assert!(!is_previewable_link("http://192.168.1.2/page"));
    }
}
