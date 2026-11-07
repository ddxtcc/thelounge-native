use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::theme::CustomTheme;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StatusMessages {
    #[default]
    Condensed,
    Shown,
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedDraft {
    pub channel: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedHistory {
    pub channel: i64,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub server: String,
    pub user: String,
    pub token: String,
    pub last_channel: Option<i64>,
    pub theme: String,
    pub font_family: String,
    pub monospace_font: String,
    pub font_size: f32,
    pub line_height: f32,
    pub message_spacing: f32,
    pub image_cache_limit: usize,
    pub show_inline_images: bool,
    pub show_previews: bool,
    pub show_sidebar: bool,
    pub nick_column: bool,
    pub show_timestamps: bool,
    pub use_12h_clock: bool,
    pub show_seconds: bool,
    pub colored_nicks: bool,
    pub status_messages: StatusMessages,
    pub show_user_list: bool,
    pub notifications: bool,
    pub icon_buttons: bool,
    pub show_tray_icon: bool,
    pub enable_global_hotkey: bool,
    pub global_hotkey: String,
    pub custom_theme: Option<CustomTheme>,
    pub favorite_commands: Vec<String>,
    pub command_aliases: HashMap<String, String>,
    pub saved_drafts: Vec<SavedDraft>,
    pub saved_histories: Vec<SavedHistory>,
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
    pub window_width: Option<f32>,
    pub window_height: Option<f32>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            user: String::new(),
            token: String::new(),
            last_channel: None,
            theme: "dark".into(),
            font_family: ".SystemUIFont".into(),
            monospace_font: "monospace".into(),
            font_size: 15.0,
            line_height: 1.45,
            message_spacing: 2.0,
            image_cache_limit: 256,
            show_inline_images: true,
            show_previews: true,
            show_sidebar: true,
            nick_column: true,
            show_timestamps: true,
            use_12h_clock: false,
            show_seconds: false,
            colored_nicks: true,
            status_messages: StatusMessages::Condensed,
            show_user_list: true,
            notifications: true,
            icon_buttons: true,
            show_tray_icon: true,
            enable_global_hotkey: true,
            global_hotkey: "control+shift+l".into(),
            custom_theme: None,
            favorite_commands: Vec::new(),
            command_aliases: HashMap::new(),
            saved_drafts: Vec::new(),
            saved_histories: Vec::new(),
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
        }
    }
}

impl AppConfig {
    pub fn path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("lounge-native").join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = Self::path();
        let Some(parent) = path.parent() else {
            return;
        };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        let Ok(text) = toml::to_string_pretty(self) else {
            return;
        };
        let temp = path.with_extension("toml.tmp");
        let result = (|| -> std::io::Result<()> {
            let mut options = fs::OpenOptions::new();
            options.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temp)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temp, &path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
    }
}
