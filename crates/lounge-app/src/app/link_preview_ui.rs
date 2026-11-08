use super::*;

impl LoungeApp {
    pub(super) fn render_link_preview_card(
        &self,
        id: &str,
        preview: &lounge_core::proto::LinkPreview,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let link = preview.link.clone();
        let thumb = if preview.thumb.is_empty() {
            None
        } else {
            Some(self.absolute_media_url(&preview.thumb))
        };
        let mut text = div()
            .flex_1()
            .min_w(px(0.))
            .py_1()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .truncate()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(preview.head.clone()),
            );
        if !preview.body.is_empty() {
            text = text.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .max_h(px(48.))
                    .overflow_hidden()
                    .child(preview.body.clone()),
            );
        }
        let mut card = div()
            .id(ElementId::Name(id.to_string().into()))
            .w_full()
            .px_2()
            .rounded(px(4.))
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .text_size(px(12.))
            .text_color(theme.text)
            .cursor_pointer()
            .hover(|style| style.border_color(theme.accent))
            .flex()
            .items_start()
            .gap_2()
            .on_click(cx.listener(move |_, _, _, _| {
                let _ = open::that(link.clone());
            }));
        if let Some(thumb) = thumb {
            card = card.child(
                div()
                    .w(px(84.))
                    .h(px(84.))
                    .flex_shrink_0()
                    .rounded(px(4.))
                    .overflow_hidden()
                    .bg(theme.sidebar)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.preview_image(&thumb, 84., cx)),
            );
        }
        card.child(text).into_any_element()
    }
}
