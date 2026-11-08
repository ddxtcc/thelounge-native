use super::*;

impl LoungeApp {
    pub(super) fn render_local_link_preview(
        &self,
        preview: &lounge_core::proto::LinkPreview,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let link = preview.link.clone();
        let mut card = div()
            .id(ElementId::Name(
                format!("local-preview-{}", preview.link).into(),
            ))
            .px_2()
            .py_1()
            .rounded(px(4.))
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .text_size(px(12.))
            .text_color(theme.text)
            .cursor_pointer()
            .hover(|style| style.border_color(theme.accent))
            .flex()
            .flex_col()
            .gap_1()
            .on_click(cx.listener(move |_, _, _, _| {
                let _ = open::that(link.clone());
            }))
            .child(div().truncate().child(preview.head.clone()));
        if !preview.body.is_empty() {
            card = card.child(
                div()
                    .truncate()
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child(preview.body.clone()),
            );
        }
        if !preview.thumb.is_empty() {
            let thumb = self.absolute_media_url(&preview.thumb);
            card = card.child(
                div()
                    .w(px(220.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.preview_image(&thumb, 220., cx)),
            );
        }
        card.into_any_element()
    }
}
