use super::*;
use crate::theme::{CustomTheme, SavedTheme, hex_rgb, parse_hex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ThemeColor {
    Bg,
    Sidebar,
    Panel,
    Hover,
    Border,
    Text,
    Muted,
    Faint,
    Accent,
    AccentDim,
    Success,
    Warning,
    Error,
    HighlightBg,
    SelfText,
    InputBg,
}

pub(super) const THEME_COLORS: &[(ThemeColor, &str)] = &[
    (ThemeColor::Bg, "Background"),
    (ThemeColor::Sidebar, "Sidebar"),
    (ThemeColor::Panel, "Panel"),
    (ThemeColor::Hover, "Hover"),
    (ThemeColor::Border, "Border"),
    (ThemeColor::Text, "Text"),
    (ThemeColor::Muted, "Muted text"),
    (ThemeColor::Faint, "Faint text"),
    (ThemeColor::Accent, "Accent"),
    (ThemeColor::AccentDim, "Accent dim"),
    (ThemeColor::Success, "Success"),
    (ThemeColor::Warning, "Warning"),
    (ThemeColor::Error, "Error"),
    (ThemeColor::HighlightBg, "Highlight"),
    (ThemeColor::SelfText, "Own text"),
    (ThemeColor::InputBg, "Input background"),
];

impl ThemeColor {
    fn value(self, theme: &CustomTheme) -> &str {
        match self {
            Self::Bg => &theme.bg,
            Self::Sidebar => &theme.sidebar,
            Self::Panel => &theme.panel,
            Self::Hover => &theme.hover,
            Self::Border => &theme.border,
            Self::Text => &theme.text,
            Self::Muted => &theme.muted,
            Self::Faint => &theme.faint,
            Self::Accent => &theme.accent,
            Self::AccentDim => &theme.accent_dim,
            Self::Success => &theme.success,
            Self::Warning => &theme.warning,
            Self::Error => &theme.error,
            Self::HighlightBg => &theme.highlight_bg,
            Self::SelfText => &theme.self_text,
            Self::InputBg => &theme.input_bg,
        }
    }

    pub(super) fn set(self, theme: &mut CustomTheme, value: String) {
        match self {
            Self::Bg => theme.bg = value,
            Self::Sidebar => theme.sidebar = value,
            Self::Panel => theme.panel = value,
            Self::Hover => theme.hover = value,
            Self::Border => theme.border = value,
            Self::Text => theme.text = value,
            Self::Muted => theme.muted = value,
            Self::Faint => theme.faint = value,
            Self::Accent => theme.accent = value,
            Self::AccentDim => theme.accent_dim = value,
            Self::Success => theme.success = value,
            Self::Warning => theme.warning = value,
            Self::Error => theme.error = value,
            Self::HighlightBg => theme.highlight_bg = value,
            Self::SelfText => theme.self_text = value,
            Self::InputBg => theme.input_bg = value,
        }
    }
}

pub(super) struct ThemeEditor {
    pub name: String,
    pub theme: CustomTheme,
    pub editing: Option<String>,
    pub selected: ThemeColor,
    pub previous: crate::theme::Theme,
    pub position: Point<Pixels>,
    pub drag_last: Option<Point<Pixels>>,
}

fn relative_luminance(color: gpui::Hsla) -> f32 {
    let color = color.to_rgb();
    let channel = |value: f32| {
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

fn contrast_ratio(first: gpui::Hsla, second: gpui::Hsla) -> f32 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    let (light, dark) = if first > second {
        (first, second)
    } else {
        (second, first)
    };
    (light + 0.05) / (dark + 0.05)
}

fn contrast_warnings(theme: &crate::theme::Theme) -> Vec<&'static str> {
    let mut warnings = Vec::new();
    if contrast_ratio(theme.text, theme.bg) < 4.5 {
        warnings.push("Text has low contrast against the background.");
    }
    if contrast_ratio(theme.text, theme.panel) < 4.5 {
        warnings.push("Text has low contrast against panels.");
    }
    if contrast_ratio(theme.on_accent, theme.accent) < 3.0 {
        warnings.push("Accent text has low contrast against the accent color.");
    }
    warnings
}

fn color_component(value: &str, component: u8) -> u8 {
    let color = parse_hex(value).unwrap_or(0);
    ((color >> ((2 - component) * 8)) & 0xff) as u8
}

fn with_component(value: &str, component: u8, channel: u8) -> String {
    let color = parse_hex(value).unwrap_or(0);
    let shift = (2 - component) * 8;
    hex_rgb((color & !(0xff << shift)) | ((channel as u32) << shift))
}

impl LoungeApp {
    pub(super) fn new_theme_editor(&mut self, cx: &mut Context<Self>) {
        if self.theme_editor.is_some() {
            self.save_theme_editor(cx);
        }
        let theme = CustomTheme::from_theme(&self.theme);
        self.open_theme_editor("Custom theme".into(), theme, None, cx);
    }

    pub(super) fn edit_current_theme(&mut self, cx: &mut Context<Self>) {
        if self.theme_editor.is_some() {
            self.save_theme_editor(cx);
        }
        if let Some(saved) = self
            .config
            .saved_themes
            .iter()
            .find(|saved| saved.id == self.config.theme)
            .cloned()
        {
            self.open_theme_editor(saved.name, saved.theme, Some(saved.id), cx);
            return;
        }
        let label = crate::theme::THEMES
            .iter()
            .find(|(id, _)| *id == self.config.theme)
            .map(|(_, label)| *label)
            .unwrap_or("Theme");
        let name = format!("{label} Custom");
        let theme = CustomTheme::from_theme(&self.theme);
        self.open_theme_editor(name, theme, None, cx);
    }

    pub(super) fn edit_selected_theme(&mut self, cx: &mut Context<Self>) {
        if self.theme_editor.is_some() {
            self.save_theme_editor(cx);
        }
        let Some(saved) = self
            .config
            .saved_themes
            .iter()
            .find(|saved| saved.id == self.config.theme)
            .cloned()
        else {
            self.toast("Select a custom theme first.");
            return;
        };
        self.open_theme_editor(saved.name, saved.theme, Some(saved.id), cx);
    }

    fn open_theme_editor(
        &mut self,
        name: String,
        theme: CustomTheme,
        editing: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.inputs.theme_name.update(cx, |input, cx| {
            input.set_content(name.clone(), cx);
        });
        self.settings_open = false;
        self.theme_editor = Some(ThemeEditor {
            name,
            theme,
            editing,
            selected: ThemeColor::Accent,
            previous: self.theme,
            position: gpui::point(px(220.), px(80.)),
            drag_last: None,
        });
        self.sync_theme_hex(cx);
        cx.notify();
    }

    pub(super) fn cancel_theme_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.theme_editor.take() else {
            return;
        };
        self.apply_theme_preview_with(editor.previous, cx);
        cx.notify();
    }

    fn sync_theme_hex(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.theme_editor.as_ref() else {
            return;
        };
        let value = editor.selected.value(&editor.theme).to_string();
        self.inputs.theme_hex.update(cx, |input, cx| {
            input.set_content(value, cx);
        });
    }

    pub(super) fn save_theme_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.theme_editor.take() else {
            return;
        };
        let name = editor.name.trim().to_string();
        let name = if name.is_empty() {
            "Custom theme".to_string()
        } else {
            name
        };
        if let Some(id) = editor.editing {
            if let Some(saved) = self
                .config
                .saved_themes
                .iter_mut()
                .find(|saved| saved.id == id)
            {
                saved.name = name;
                saved.theme = editor.theme;
                self.config.theme = id;
            }
        } else {
            let saved = SavedTheme::new(name, editor.theme);
            self.config.theme = saved.id.clone();
            self.config.saved_themes.push(saved);
        }
        self.apply_theme(cx);
        self.config.save();
        self.toast("Theme saved.");
        cx.notify();
    }

    pub(super) fn delete_theme_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.theme_editor.take() else {
            return;
        };
        let Some(id) = editor.editing else {
            cx.notify();
            return;
        };
        self.config.saved_themes.retain(|saved| saved.id != id);
        if self.config.theme == id {
            self.config.theme = "dark".into();
        }
        self.apply_theme(cx);
        self.config.save();
        self.toast("Theme deleted.");
        cx.notify();
    }

    pub(super) fn render_theme_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(editor) = self.theme_editor.as_ref() else {
            return div().into_any_element();
        };
        let preview = editor.theme.to_theme();
        let selected = editor.selected;
        let mut rows = div().flex().flex_wrap().gap_2();
        for (color, label) in THEME_COLORS {
            let color_value = parse_hex(color.value(&editor.theme)).unwrap_or(0);
            let active = *color == selected;
            rows = rows.child(
                div()
                    .id(ElementId::Name(format!("theme-color-{label}").into()))
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_1()
                    .rounded(px(4.))
                    .border_1()
                    .border_color(if active {
                        self.theme.accent
                    } else {
                        self.theme.border
                    })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(editor) = this.theme_editor.as_mut() {
                            editor.selected = *color;
                        }
                        this.sync_theme_hex(cx);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .size(px(12.))
                            .rounded(px(3.))
                            .bg(gpui::Hsla::from(gpui::rgb(color_value))),
                    )
                    .child(div().text_size(px(11.)).child(*label)),
            );
        }
        let selected_value = selected.value(&editor.theme);
        let r = color_component(selected_value, 0);
        let g = color_component(selected_value, 1);
        let b = color_component(selected_value, 2);
        let warnings = contrast_warnings(&preview);
        let mut content = div()
            .w_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .child(self.labeled_input("Theme name", &self.inputs.theme_name))
            .child(self.setting_section("Colors"))
            .child(rows)
            .child(self.setting_section("Selected color"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .w(px(70.))
                            .text_size(px(12.))
                            .text_color(self.theme.muted)
                            .child("Hex code"),
                    )
                    .child(div().flex_1().child(self.inputs.theme_hex.clone())),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div().text_size(px(11.)).text_color(self.theme.muted).child(
                            *THEME_COLORS
                                .iter()
                                .find(|(color, _)| *color == selected)
                                .map(|(_, label)| label)
                                .unwrap_or(&"Color"),
                        ),
                    )
                    .child(self.theme_color_slider("R", r, 0, cx))
                    .child(self.theme_color_slider("G", g, 1, cx))
                    .child(self.theme_color_slider("B", b, 2, cx)),
            )
            .child(
                div()
                    .p_3()
                    .rounded(px(5.))
                    .bg(preview.bg)
                    .border_1()
                    .border_color(preview.border)
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(preview.text)
                            .child("Preview message"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(preview.muted)
                            .child("Muted text and timestamps"),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded(px(4.))
                            .bg(preview.accent)
                            .text_color(preview.on_accent)
                            .child("Accent button"),
                    ),
            )
            .child(
                self.setting_row_button("Cancel", "Discard", cx, |this, cx| {
                    this.cancel_theme_editor(cx)
                }),
            )
            .when(editor.editing.is_some(), |this| {
                this.child(
                    self.setting_row_button("Delete theme", "Delete", cx, |this, cx| {
                        this.delete_theme_editor(cx)
                    }),
                )
            });
        for warning in warnings {
            content = content.child(
                div()
                    .text_size(px(11.))
                    .text_color(self.theme.warning)
                    .child(warning),
            );
        }
        div()
            .id("theme-editor-body")
            .w_full()
            .max_h(px(700.))
            .overflow_y_scroll()
            .rounded(px(8.))
            .border_1()
            .border_color(self.theme.border)
            .bg(self.theme.panel)
            .child(content)
            .into_any_element()
    }

    pub(super) fn render_theme_editor_window(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(editor) = self.theme_editor.as_ref() else {
            return div().into_any_element();
        };
        let position = editor.position;
        div()
            .absolute()
            .left(position.x)
            .top(position.y)
            .w(px(580.))
            .max_h(px(760.))
            .occlude()
            .child(
                div()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(self.theme.border)
                    .bg(self.theme.panel)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id("theme-editor-header")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_2()
                            .border_b_1()
                            .border_color(self.theme.border)
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                                    if let Some(editor) = this.theme_editor.as_mut() {
                                        editor.drag_last = Some(event.position);
                                    }
                                    cx.notify();
                                }),
                            )
                            .on_mouse_move(cx.listener(
                                |this, event: &gpui::MouseMoveEvent, _, cx| {
                                    if event.pressed_button != Some(MouseButton::Left) {
                                        return;
                                    }
                                    if let Some(editor) = this.theme_editor.as_mut()
                                        && let Some(last) = editor.drag_last
                                    {
                                        editor.position.x += event.position.x - last.x;
                                        editor.position.y += event.position.y - last.y;
                                        editor.drag_last = Some(event.position);
                                        cx.notify();
                                    }
                                },
                            ))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    if let Some(editor) = this.theme_editor.as_mut() {
                                        editor.drag_last = None;
                                    }
                                    cx.notify();
                                }),
                            )
                            .child(div().text_size(px(13.)).child("Theme editor"))
                            .child(
                                div()
                                    .id("theme-editor-close")
                                    .size(px(24.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(self.theme.hover))
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.save_theme_editor(cx)),
                                    )
                                    .child(
                                        gpui::svg()
                                            .path("icons/x.svg")
                                            .w(px(14.))
                                            .h(px(14.))
                                            .text_color(self.theme.muted),
                                    ),
                            ),
                    )
                    .child(self.render_theme_editor(cx)),
            )
            .into_any_element()
    }

    fn theme_color_slider(
        &self,
        label: &str,
        value: u8,
        component: u8,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let bounds = std::rc::Rc::new(std::cell::RefCell::new(None::<Bounds<Pixels>>));
        let click_bounds = bounds.clone();
        let move_bounds = bounds.clone();
        let fraction = value as f32 / 255.;
        let theme = self.theme;
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(12.))
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child(label.to_string()),
            )
            .child(
                div()
                    .h(px(9.))
                    .flex_1()
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .on_children_prepainted(move |rects, _, _| {
                        *bounds.borrow_mut() = rects.first().copied();
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                            Self::set_theme_component(
                                this,
                                *click_bounds.borrow(),
                                event.position.x,
                                component,
                                cx,
                            );
                        }),
                    )
                    .on_mouse_move(
                        cx.listener(move |this, event: &gpui::MouseMoveEvent, _, cx| {
                            if event.pressed_button == Some(MouseButton::Left) {
                                Self::set_theme_component(
                                    this,
                                    *move_bounds.borrow(),
                                    event.position.x,
                                    component,
                                    cx,
                                );
                            }
                        }),
                    )
                    .child(
                        div()
                            .h(px(3.))
                            .w_full()
                            .rounded_full()
                            .bg(theme.border)
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(fraction))
                                    .rounded_full()
                                    .bg(theme.accent),
                            ),
                    ),
            )
            .child(
                div()
                    .w(px(24.))
                    .text_size(px(11.))
                    .text_color(theme.muted)
                    .child(value.to_string()),
            )
            .into_any_element()
    }

    fn set_theme_component(
        this: &mut Self,
        bounds: Option<Bounds<Pixels>>,
        x: Pixels,
        component: u8,
        cx: &mut Context<Self>,
    ) {
        let Some(bounds) = bounds else {
            return;
        };
        if bounds.size.width <= px(0.) {
            return;
        }
        let fraction = ((x - bounds.origin.x) / bounds.size.width).clamp(0., 1.);
        {
            let Some(editor) = this.theme_editor.as_mut() else {
                return;
            };
            let value = (fraction * 255.).round() as u8;
            let current = editor.selected.value(&editor.theme).to_string();
            editor.selected.set(
                &mut editor.theme,
                with_component(&current, component, value),
            );
        }
        this.sync_theme_hex(cx);
        this.apply_theme_preview(cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn color_components_round_trip() {
        let color = "#123456";
        assert_eq!(color_component(color, 0), 0x12);
        assert_eq!(color_component(color, 1), 0x34);
        assert_eq!(color_component(color, 2), 0x56);
        assert_eq!(with_component(color, 0, 0xab), "#ab3456");
        assert_eq!(with_component(color, 1, 0xcd), "#12cd56");
        assert_eq!(with_component(color, 2, 0xef), "#1234ef");
    }

    #[test]
    fn contrast_ratio_distinguishes_black_and_white() {
        let black = gpui::Hsla::from(gpui::rgb(0x000000));
        let white = gpui::Hsla::from(gpui::rgb(0xffffff));
        assert!((contrast_ratio(black, white) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(black, black) - 1.0).abs() < 0.01);
        assert!(contrast_warnings(&Theme::dark()).is_empty());
    }

    #[test]
    fn every_theme_color_maps_to_a_field() {
        let mut custom = CustomTheme::from_theme(&Theme::dark());
        for (color, _) in THEME_COLORS {
            color.set(&mut custom, "#010203".into());
            assert_eq!(color.value(&custom), "#010203");
        }
    }
}
