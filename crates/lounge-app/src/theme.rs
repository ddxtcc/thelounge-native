use gpui::{Hsla, rgb};

fn contrast_text(color: u32) -> Hsla {
    let r = ((color >> 16) & 0xff) as f32 / 255.0;
    let g = ((color >> 8) & 0xff) as f32 / 255.0;
    let b = (color & 0xff) as f32 / 255.0;
    let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    if luminance > 0.55 {
        rgb(0x0c0e12).into()
    } else {
        rgb(0xffffff).into()
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    pub bg: Hsla,
    pub sidebar: Hsla,
    pub panel: Hsla,
    pub hover: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub faint: Hsla,
    pub accent: Hsla,
    pub accent_dim: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    pub highlight_bg: Hsla,
    pub self_text: Hsla,
    pub input_bg: Hsla,
    pub on_accent: Hsla,
    pub overlay: Hsla,
}

pub const THEMES: &[(&str, &str)] = &[
    ("dark", "Dark"),
    ("light", "Light"),
    ("nord", "Nord"),
    ("dracula", "Dracula"),
    ("gruvbox", "Gruvbox"),
    ("tokyo-night", "Tokyo Night"),
    ("solarized-light", "Solarized Light"),
    ("oled", "OLED Black"),
];

impl Theme {
    pub fn from_mode(mode: &str) -> Self {
        match mode {
            "light" => Self::light(),
            "nord" => Self::nord(),
            "dracula" => Self::dracula(),
            "gruvbox" => Self::gruvbox(),
            "tokyo-night" => Self::tokyo_night(),
            "solarized-light" => Self::solarized_light(),
            "oled" => Self::oled(),
            _ => Self::dark(),
        }
    }

    pub fn dark() -> Self {
        Self::palette(
            true, 0x111318, 0x0c0e12, 0x171a21, 0x1f2430, 0x262a33, 0xe6e8ee, 0x8b93a3, 0x5b6270,
            0x7aa2f7, 0x2c3650, 0x9ece6a, 0xe0af68, 0xf7768e, 0x2a2233, 0x9aa2b1, 0x1b1e26,
        )
    }

    pub fn light() -> Self {
        Self::palette(
            false, 0xf7f7fa, 0xececf3, 0xffffff, 0xe3e5ec, 0xd7d9e0, 0x20232a, 0x6b7280, 0x9aa0ab,
            0x3b6fd4, 0xdbe4f8, 0x2f8f46, 0xa86b00, 0xc02b46, 0xeee6f6, 0x555b66, 0xffffff,
        )
    }

    pub fn nord() -> Self {
        Self::palette(
            true, 0x2e3440, 0x272c36, 0x3b4252, 0x434c5e, 0x4c566a, 0xeceff4, 0xa3abbd, 0x6d7689,
            0x88c0d0, 0x3f5666, 0xa3be8c, 0xebcb8b, 0xbf616a, 0x4a3b4f, 0xb8c0d0, 0x343b49,
        )
    }

    pub fn dracula() -> Self {
        Self::palette(
            true, 0x282a36, 0x21222c, 0x343746, 0x44475a, 0x44475a, 0xf8f8f2, 0x9aa0b5, 0x6272a4,
            0xbd93f9, 0x4a3a68, 0x50fa7b, 0xf1fa8c, 0xff5555, 0x4b3a65, 0xb8b8c8, 0x30323f,
        )
    }

    pub fn gruvbox() -> Self {
        Self::palette(
            true, 0x282828, 0x1d2021, 0x3c3836, 0x504945, 0x504945, 0xebdbb2, 0xa89984, 0x7c6f64,
            0xfabd2f, 0x5a4a2a, 0xb8bb26, 0xfabd2f, 0xfb4934, 0x4a3b2a, 0xd5c4a1, 0x32302f,
        )
    }

    pub fn tokyo_night() -> Self {
        Self::palette(
            true, 0x1a1b26, 0x16161e, 0x24283b, 0x2f3549, 0x3b4261, 0xc0caf5, 0x9aa5ce, 0x565f89,
            0x7aa2f7, 0x2d3f66, 0x9ece6a, 0xe0af68, 0xf7768e, 0x33294a, 0xa9b1d6, 0x1f2335,
        )
    }

    pub fn oled() -> Self {
        Self::palette(
            true, 0x000000, 0x000000, 0x0a0a0a, 0x161616, 0x262626, 0xf2f2f2, 0x9a9a9a, 0x5f5f5f,
            0x7aa2f7, 0x1e2a44, 0x9ece6a, 0xe0af68, 0xf7768e, 0x1b1424, 0xa8a8a8, 0x050505,
        )
    }

    pub fn solarized_light() -> Self {
        Self::palette(
            false, 0xfdf6e3, 0xeee8d5, 0xfffbef, 0xe6e0cd, 0xd5cdb8, 0x586e75, 0x93a1a1, 0xb0a999,
            0x268bd2, 0xd9e6f2, 0x859900, 0xb58900, 0xdc322f, 0xf5e6d3, 0x657b83, 0xfffdf5,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn palette(
        dark: bool,
        bg: u32,
        sidebar: u32,
        panel: u32,
        hover: u32,
        border: u32,
        text: u32,
        muted: u32,
        faint: u32,
        accent: u32,
        accent_dim: u32,
        success: u32,
        warning: u32,
        error: u32,
        highlight_bg: u32,
        self_text: u32,
        input_bg: u32,
    ) -> Self {
        Self {
            dark,
            bg: rgb(bg).into(),
            sidebar: rgb(sidebar).into(),
            panel: rgb(panel).into(),
            hover: rgb(hover).into(),
            border: rgb(border).into(),
            text: rgb(text).into(),
            muted: rgb(muted).into(),
            faint: rgb(faint).into(),
            accent: rgb(accent).into(),
            accent_dim: rgb(accent_dim).into(),
            success: rgb(success).into(),
            warning: rgb(warning).into(),
            error: rgb(error).into(),
            highlight_bg: rgb(highlight_bg).into(),
            self_text: rgb(self_text).into(),
            input_bg: rgb(input_bg).into(),
            on_accent: contrast_text(accent),
            overlay: gpui::Hsla {
                a: if dark { 0.6 } else { 0.35 },
                ..rgb(0x000000).into()
            },
        }
    }

    pub fn input_colors(&self) -> crate::text_input::InputColors {
        crate::text_input::InputColors {
            text: self.text,
            background: self.input_bg,
            border: self.border,
            focus_border: self.accent,
        }
    }
}
