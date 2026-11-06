use super::*;

impl LoungeApp {
    pub(super) fn render_palette(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let entries = self.palette_entries(cx);
        let selection = self.palette_selection.min(entries.len().saturating_sub(1));
        let title = match self.palette {
            Some(PaletteMode::Commands) => "Command palette",
            Some(PaletteMode::Channels) => "Switch channel",
            None => "Command palette",
        };
        let mut list = div().flex().flex_col().gap_0p5().w_full();
        if entries.is_empty() {
            list = list.child(div().p_2().text_color(theme.muted).child("No matches."));
        }
        for (index, (label, entry)) in entries.into_iter().enumerate() {
            let action = entry.clone();
            list =
                list.child(
                    div()
                        .id(ElementId::Name(format!("palette-{index}").into()))
                        .px_2()
                        .py_1()
                        .rounded(px(4.))
                        .cursor_pointer()
                        .text_color(if index == selection {
                            theme.accent
                        } else {
                            theme.text
                        })
                        .when(index == selection, |this| this.bg(theme.hover))
                        .hover(|style| style.bg(theme.hover))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.run_palette_entry(action.clone(), cx)
                        }))
                        .child(label),
                );
        }
        self.modal_card(title, cx)
            .child(self.inputs.palette.clone())
            .child(list)
            .into_any_element()
    }
}
