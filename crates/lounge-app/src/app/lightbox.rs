use super::*;

impl LoungeApp {
    pub(super) fn render_lightbox(
        &self,
        url: &str,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        #[cfg(feature = "media")]
        if let Some(kind) = super::media::media_kind(url) {
            return self.render_media_lightbox(url, kind, window, cx);
        }
        let theme = self.theme;
        self.touch_image(url);
        let viewport = window.viewport_size();
        let max_w = viewport.width * 0.88;
        let max_h = viewport.height * 0.82;
        let open_url = url.to_string();
        let zoom = self.lightbox_zoom;
        let pan = self.lightbox_pan;
        div()
            .id("lightbox")
            .max_w(max_w)
            .max_h(max_h)
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(
                div()
                    .id("lightbox-image")
                    .w(max_w)
                    .h(max_h)
                    .relative()
                    .overflow_hidden()
                    .rounded(px(6.))
                    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                            this.lightbox_drag_last = Some(event.position);
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.lightbox_drag_last = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                        if !event.dragging() {
                            return;
                        }
                        if let Some(last) = this.lightbox_drag_last {
                            this.lightbox_pan.x += event.position.x - last.x;
                            this.lightbox_pan.y += event.position.y - last.y;
                            this.lightbox_drag_last = Some(event.position);
                            cx.notify();
                        }
                    }))
                    .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _, cx| {
                        let delta = event.delta.pixel_delta(px(20.)).y;
                        let factor = if delta > px(0.) { 0.9 } else { 1.1 };
                        this.lightbox_zoom = (this.lightbox_zoom * factor).clamp(0.5, 5.0);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .absolute()
                            .left(pan.x)
                            .top(pan.y)
                            .w(max_w * zoom)
                            .h(max_h * zoom)
                            .child(
                                gpui::img(url.to_string())
                                    .image_cache(&self.image_cache)
                                    .w_full()
                                    .h_full()
                                    .object_fit(gpui::ObjectFit::Contain)
                                    .with_fallback(move || {
                                        div()
                                            .w_full()
                                            .h_full()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_color(theme.muted)
                                            .child("Image failed to load")
                                            .into_any_element()
                                    })
                                    .rounded(px(6.)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.muted)
                            .child("Scroll to zoom · Drag to pan · Click outside to close"),
                    )
                    .when(self.lightbox_index.is_some(), |this| {
                        this.child(
                            div()
                                .id("lightbox-prev")
                                .tab_index(0)
                                .focus(|style| style.bg(theme.border).text_color(theme.text))
                                .px_1()
                                .text_size(px(11.))
                                .text_color(theme.accent)
                                .cursor_pointer()
                                .hover(|style| style.underline())
                                .on_click(cx.listener(|this, _, _, cx| this.move_lightbox(-1, cx)))
                                .on_key_down(cx.listener(
                                    |this, event: &gpui::KeyDownEvent, _, cx| {
                                        if activation_key(event) {
                                            this.move_lightbox(-1, cx);
                                            cx.stop_propagation();
                                        }
                                    },
                                ))
                                .child("Previous"),
                        )
                        .child(
                            div()
                                .id("lightbox-next")
                                .tab_index(0)
                                .focus(|style| style.bg(theme.border).text_color(theme.text))
                                .px_1()
                                .text_size(px(11.))
                                .text_color(theme.accent)
                                .cursor_pointer()
                                .hover(|style| style.underline())
                                .on_click(cx.listener(|this, _, _, cx| this.move_lightbox(1, cx)))
                                .on_key_down(cx.listener(
                                    |this, event: &gpui::KeyDownEvent, _, cx| {
                                        if activation_key(event) {
                                            this.move_lightbox(1, cx);
                                            cx.stop_propagation();
                                        }
                                    },
                                ))
                                .child("Next"),
                        )
                    })
                    .child({
                        let click_url = open_url.clone();
                        div()
                            .id("lightbox-open")
                            .tab_index(0)
                            .focus(|style| style.bg(theme.border).text_color(theme.text))
                            .text_size(px(11.))
                            .text_color(theme.accent)
                            .cursor_pointer()
                            .hover(|style| style.underline())
                            .on_click(cx.listener(move |_, _, _, _| {
                                let _ = open::that(click_url.clone());
                            }))
                            .on_key_down({
                                let open_url = open_url.clone();
                                cx.listener(move |_, event: &gpui::KeyDownEvent, _, cx| {
                                    if activation_key(event) {
                                        let _ = open::that(open_url.clone());
                                        cx.stop_propagation();
                                    }
                                })
                            })
                            .child("Open in browser")
                    }),
            )
            .into_any_element()
    }
}
