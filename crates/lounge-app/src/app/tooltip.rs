use super::*;

pub(super) struct Tooltip {
    label: SharedString,
    theme: Theme,
}

impl Tooltip {
    pub(super) fn new(label: impl Into<SharedString>, theme: Theme) -> Self {
        Self {
            label: label.into(),
            theme,
        }
    }
}

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded(px(4.))
            .bg(self.theme.panel)
            .border_1()
            .border_color(self.theme.border)
            .text_size(px(11.))
            .text_color(self.theme.text)
            .child(self.label.clone())
    }
}
