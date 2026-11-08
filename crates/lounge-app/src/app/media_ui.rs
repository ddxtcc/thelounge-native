use super::media_kind::MediaKind;
use super::tooltip::Tooltip;
use super::*;

impl LoungeApp {
    pub(super) fn media_icon_button(
        &self,
        icon: &str,
        tooltip: &str,
        active: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let tooltip = tooltip.to_string();
        let color = if active { theme.accent } else { theme.muted };
        let icon = icon.to_string();
        div()
            .id(ElementId::Name(format!("media-{icon}-{tooltip}").into()))
            .tab_index(0)
            .tab_stop(!self.last_overlay_open || self.lightbox.is_some())
            .focus(|style| style.bg(theme.border).text_color(theme.text))
            .size(px(26.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .cursor_pointer()
            .when(active, |this| this.bg(theme.accent_dim))
            .when(!active, |this| this.hover(|style| style.bg(theme.hover)))
            .on_click(cx.listener(move |this, _, window, cx| {
                on_click(this, window, cx);
                cx.stop_propagation();
            }))
            .tooltip(move |_, cx| cx.new(|_| Tooltip::new(tooltip.clone(), theme)).into())
            .child(
                gpui::svg()
                    .path(format!("icons/{icon}.svg"))
                    .w(px(16.))
                    .h(px(16.))
                    .text_color(color),
            )
            .into_any_element()
    }

    pub(super) fn toggle_media_fullscreen(&mut self, cx: &mut Context<Self>) {
        if self.media_fullscreen {
            self.media_fullscreen = false;
            cx.notify();
            return;
        }
        if let Some(inline) = self.inline_video.take() {
            self.lightbox = Some(inline.url.clone());
            self.media_video = Some((inline.url, inline.video));
        } else if let Some(inline) = self.inline_audio.take() {
            self.lightbox = Some(inline.url.clone());
            self.media_audio_url = Some(inline.url);
            self.media_audio = Some(inline.pipeline);
        }
        self.media_fullscreen = self.media_video.is_some() || self.media_audio.is_some();
        cx.notify();
    }

    pub(super) fn render_audio_preview(
        &self,
        message_id: i64,
        url: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let play_url = url.to_string();
        let expand_url = url.to_string();
        let download_url = url.to_string();
        let file = url.rsplit('/').next().unwrap_or(url).to_string();
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        gpui::svg()
                            .path("icons/music.svg")
                            .w(px(14.))
                            .h(px(14.))
                            .text_color(self.theme.accent),
                    )
                    .child(div().flex_1().truncate().child(file)),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .h(px(5.))
                            .w_full()
                            .rounded_full()
                            .bg(self.theme.border),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(10.))
                            .text_color(self.theme.muted)
                            .child("0:00 / --:--")
                            .child(
                                div()
                                    .w(px(54.))
                                    .h(px(3.))
                                    .rounded_full()
                                    .bg(self.theme.border),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap_1()
                            .child(self.media_icon_button(
                                "play",
                                "Play",
                                false,
                                cx,
                                move |this, _, cx| {
                                    this.play_inline_media(
                                        message_id,
                                        play_url.clone(),
                                        MediaKind::Audio,
                                        cx,
                                    );
                                },
                            ))
                            .child(self.media_icon_button(
                                if self.audio_start_muted {
                                    "volume-x"
                                } else {
                                    "volume-2"
                                },
                                if self.audio_start_muted {
                                    "Unmute"
                                } else {
                                    "Mute"
                                },
                                false,
                                cx,
                                |this, _, cx| {
                                    let muted = !this.audio_start_muted;
                                    this.audio_start_muted = muted;
                                    this.config.media_muted = muted;
                                    this.config.save();
                                    cx.notify();
                                },
                            ))
                            .child(self.media_icon_button(
                                "expand",
                                "Expand",
                                false,
                                cx,
                                move |this, _, cx| this.open_lightbox(expand_url.clone(), cx),
                            ))
                            .child(self.media_icon_button(
                                "download",
                                "Download",
                                false,
                                cx,
                                move |this, _, cx| this.download_media(download_url.clone(), cx),
                            )),
                    ),
            )
            .into_any_element()
    }
    pub(super) fn media_fullscreen_button(&self, cx: &mut Context<Self>) -> AnyElement {
        self.media_icon_button(
            "maximize",
            "Full screen",
            self.media_fullscreen,
            cx,
            |this, _, cx| {
                this.toggle_media_fullscreen(cx);
            },
        )
    }
}
