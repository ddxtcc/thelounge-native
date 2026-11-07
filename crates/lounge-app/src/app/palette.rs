use super::*;

impl LoungeApp {
    pub(super) fn render_palette(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        if self.palette == Some(PaletteMode::Hotkey) {
            return self
                .modal_card("Global shortcut", cx)
                .child(
                    div()
                        .text_size(px(13.))
                        .child("Press the new shortcut. Include Ctrl, Alt, or Meta."),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.muted)
                        .child(format!("Current: {}", self.config.global_hotkey)),
                )
                .child(self.menu_button("Cancel", cx, |this, _, cx| {
                    this.cancel_hotkey_capture(cx);
                }))
                .into_any_element();
        }
        let entries = self.palette_entries(cx);
        let selection = self.palette_selection.min(entries.len().saturating_sub(1));
        let title = match self.palette {
            Some(PaletteMode::Commands) => "Command palette",
            Some(PaletteMode::Channels) => "Switch channel",
            Some(PaletteMode::Themes) => "Choose theme",
            Some(PaletteMode::Alias) => "Add command alias",
            Some(PaletteMode::Hotkey) => "Global shortcut",
            Some(PaletteMode::Users(action)) => action.title(),
            None => "Command palette",
        };
        let mut list = div()
            .id("palette-list")
            .flex()
            .flex_col()
            .gap_0p5()
            .w_full()
            .max_h(px(420.))
            .overflow_y_scroll()
            .track_scroll(&self.palette_scroll);
        if entries.is_empty() {
            list = list.child(div().p_2().text_color(theme.muted).child("No matches."));
        }
        let mut last_section = None;
        for (index, (label, entry)) in entries.into_iter().enumerate() {
            let section = match &entry {
                PaletteEntry::Recent(_) => Some("Recent"),
                PaletteEntry::Command(command)
                    if matches!(self.palette, Some(PaletteMode::Commands)) =>
                {
                    Some(command.section())
                }
                _ => None,
            };
            let show_section = section.is_some() && last_section != section;
            last_section = section;
            let action = entry.clone();
            let mut item = div()
                .id(ElementId::Name(format!("palette-{index}").into()))
                .px_2()
                .py_1()
                .rounded(px(4.))
                .cursor_pointer()
                .flex()
                .flex_col()
                .gap_0p5()
                .text_color(if index == selection {
                    theme.accent
                } else {
                    theme.text
                })
                .when(index == selection, |this| this.bg(theme.hover))
                .hover(|style| style.bg(theme.hover))
                .on_click(
                    cx.listener(move |this, _, _, cx| this.run_palette_entry(action.clone(), cx)),
                );
            if show_section {
                item = item.child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme.muted)
                        .child(section.unwrap_or_default()),
                );
            }
            let command = match &entry {
                PaletteEntry::Command(command)
                | PaletteEntry::Recent(command)
                | PaletteEntry::Favorite(command) => Some(*command),
                _ => None,
            };
            let mut text = div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .child(label);
            if let Some(command) = command {
                let favorite = self.command_is_favorite(command);
                text = text.child(
                    div()
                        .id(ElementId::Name(format!("palette-favorite-{index}").into()))
                        .px_1()
                        .text_color(if favorite { theme.accent } else { theme.muted })
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_command_favorite(command, cx);
                        }))
                        .child(if favorite { "★" } else { "☆" }),
                );
            }
            item = item.child(text);
            list = list.child(item);
        }
        self.modal_card(title, cx)
            .child(self.inputs.palette.clone())
            .child(list)
            .into_any_element()
    }
}
