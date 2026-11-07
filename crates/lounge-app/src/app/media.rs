use super::*;

pub(super) struct InlineMedia {
    pub message_id: i64,
    pub url: String,
    pub kind: MediaKind,
    pub video: gpui_video_player::Video,
}

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
        self.inline_video = None;
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

    pub(super) fn clear_inline_media(&mut self) {
        self.inline_video = None;
    }

    pub(super) fn play_inline_media(
        &mut self,
        message_id: i64,
        url: String,
        kind: MediaKind,
        cx: &mut Context<Self>,
    ) {
        if self
            .inline_video
            .as_ref()
            .is_some_and(|inline| inline.url == url)
        {
            return;
        }
        self.inline_video = None;
        let Ok(parsed) = url::Url::parse(&url) else {
            self.toast("Invalid media URL.");
            return;
        };
        match gpui_video_player::Video::new(&parsed) {
            Ok(video) => {
                video.set_looping(true);
                self.inline_video = Some(InlineMedia {
                    message_id,
                    url,
                    kind,
                    video,
                });
            }
            Err(error) => self.toast(format!("Could not play media: {error}")),
        }
        cx.notify();
    }

    pub(super) fn render_inline_media(
        &self,
        message_id: i64,
        url: &str,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let inline = self
            .inline_video
            .as_ref()
            .filter(|inline| inline.message_id == message_id && inline.url == url)?;
        let url = inline.url.clone();
        let kind = inline.kind;
        let video = inline.video.clone();
        let toggle = video.clone();
        let mute = video.clone();
        let open_url = url.clone();
        let paused = video.paused();
        let muted = video.muted();
        let mut card = div()
            .id(ElementId::Name(format!("inline-media-{url}").into()))
            .w_full()
            .max_w(px(460.))
            .rounded(px(6.))
            .bg(self.theme.panel)
            .border_1()
            .border_color(self.theme.border)
            .p_2()
            .flex()
            .flex_col()
            .gap_2();
        if kind == MediaKind::Video {
            card = card.child(
                gpui_video_player::video(video)
                    .id(ElementId::Name(format!("inline-video-{url}").into()))
                    .buffer_capacity(30)
                    .size(px(440.), px(248.)),
            );
        } else {
            card = card.child(
                div()
                    .text_size(px(12.))
                    .text_color(self.theme.text)
                    .child(url.rsplit('/').next().unwrap_or(url.as_str()).to_string()),
            );
        }
        card = card.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .id("inline-media-toggle")
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .bg(self.theme.sidebar)
                        .cursor_pointer()
                        .hover(|style| style.bg(self.theme.hover))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            toggle.set_paused(!toggle.paused());
                            cx.notify();
                        }))
                        .child(if paused { "Play" } else { "Pause" }),
                )
                .child(
                    div()
                        .id("inline-media-mute")
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .bg(self.theme.sidebar)
                        .cursor_pointer()
                        .hover(|style| style.bg(self.theme.hover))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            mute.set_muted(!mute.muted());
                            cx.notify();
                        }))
                        .child(if muted { "Unmute" } else { "Mute" }),
                )
                .child(
                    div()
                        .id("inline-media-open")
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .text_color(self.theme.accent)
                        .cursor_pointer()
                        .hover(|style| style.underline())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_lightbox(open_url.clone(), cx);
                        }))
                        .child("Open"),
                )
                .child(
                    div()
                        .id("inline-media-close")
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .text_color(self.theme.muted)
                        .cursor_pointer()
                        .hover(|style| style.text_color(self.theme.text))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.clear_inline_media();
                            cx.notify();
                        }))
                        .child("Close"),
                ),
        );
        Some(card.into_any_element())
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
