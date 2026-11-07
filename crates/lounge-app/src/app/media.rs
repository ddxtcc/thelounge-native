use super::*;

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

impl LoungeApp {
    pub(super) fn open_media_lightbox(&mut self, url: String, cx: &mut Context<Self>) {
        self.lightbox_index = None;
        self.lightbox = Some(url.clone());
        self.lightbox_zoom = 1.0;
        self.lightbox_pan = Point::default();
        self.lightbox_drag_last = None;
        self.media_video = None;
        self.media_error = None;
        let parsed = match url::Url::parse(&url) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.media_error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        match gpui_video_player::Video::new(&parsed) {
            Ok(video) => {
                video.set_looping(true);
                self.media_video = Some((url, video));
            }
            Err(error) => self.media_error = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn clear_media(&mut self) {
        self.media_video = None;
        self.media_error = None;
    }

    pub(super) fn render_media_lightbox(
        &self,
        url: &str,
        kind: MediaKind,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let viewport = window.viewport_size();
        let max_w = viewport.width * 0.88;
        let max_h = viewport.height * 0.78;
        if let Some(error) = &self.media_error {
            return div()
                .max_w(max_w)
                .p_4()
                .rounded(px(6.))
                .bg(theme.panel)
                .border_1()
                .border_color(theme.error)
                .text_color(theme.text)
                .child(format!("Could not play media: {error}"))
                .into_any_element();
        }
        let Some((_, video)) = self
            .media_video
            .as_ref()
            .filter(|(current, _)| current == url)
        else {
            return div()
                .max_w(max_w)
                .p_4()
                .rounded(px(6.))
                .bg(theme.panel)
                .text_color(theme.muted)
                .child("Loading media...")
                .into_any_element();
        };
        let video = video.clone();
        let controls_video = video.clone();
        let mute_video = video.clone();
        let paused = video.paused();
        let muted = video.muted();
        let mut body = div()
            .id("lightbox-media")
            .max_w(max_w)
            .max_h(max_h)
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()));
        if kind == MediaKind::Video {
            body = body.child(
                gpui_video_player::video(video)
                    .id("lightbox-video")
                    .buffer_capacity(30)
                    .size(max_w, max_h),
            );
        } else {
            body = body.child(
                div()
                    .w(max_w)
                    .py_12()
                    .rounded(px(6.))
                    .bg(theme.panel)
                    .border_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text)
                    .child("Audio"),
            );
        }
        body = body.child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .id("media-toggle")
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .bg(theme.panel)
                        .border_1()
                        .border_color(theme.border)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            controls_video.set_paused(!controls_video.paused());
                            cx.notify();
                        }))
                        .child(if paused { "Play" } else { "Pause" }),
                )
                .child(
                    div()
                        .id("media-mute")
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .bg(theme.panel)
                        .border_1()
                        .border_color(theme.border)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            mute_video.set_muted(!mute_video.muted());
                            cx.notify();
                        }))
                        .child(if muted { "Unmute" } else { "Mute" }),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.muted)
                        .child("Click outside to close"),
                ),
        );
        body.into_any_element()
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
