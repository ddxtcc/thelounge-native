use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::theme::{CustomTheme, SavedTheme};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewMode {
    #[default]
    Shown,
    Collapsed,
    Hidden,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationMode {
    #[default]
    Mentions,
    All,
    DirectMessages,
}

impl NotificationMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mentions => "Mentions & highlights",
            Self::All => "All messages",
            Self::DirectMessages => "Direct messages",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Mentions => Self::All,
            Self::All => Self::DirectMessages,
            Self::DirectMessages => Self::Mentions,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationOverride {
    pub channel: i64,
    pub mode: NotificationMode,
}

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
    #[serde(default, skip_serializing)]
    pub token: String,
    #[serde(skip)]
    pub token_error: Option<String>,
    pub last_channel: Option<i64>,
    pub theme: String,
    pub font_family: String,
    pub monospace_font: String,
    pub font_size: f32,
    pub line_height: f32,
    pub message_spacing: f32,
    pub image_cache_limit: usize,
    pub show_inline_images: bool,
    pub preview_mode: PreviewMode,
    pub show_previews: bool,
    pub video_thumbnails: bool,
    #[serde(default = "default_media_volume")]
    pub media_volume: f32,
    pub media_muted: bool,
    pub show_sidebar: bool,
    pub nick_column: bool,
    pub show_timestamps: bool,
    pub use_12h_clock: bool,
    pub show_seconds: bool,
    pub colored_nicks: bool,
    pub status_messages: StatusMessages,
    pub show_user_list: bool,
    pub notifications: bool,
    pub notification_mode: NotificationMode,
    pub notification_overrides: Vec<NotificationOverride>,
    pub icon_buttons: bool,
    pub show_tray_icon: bool,
    pub enable_global_hotkey: bool,
    pub global_hotkey: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_theme: Option<CustomTheme>,
    #[serde(default)]
    pub saved_themes: Vec<SavedTheme>,
    pub favorite_commands: Vec<String>,
    pub command_aliases: HashMap<String, String>,
    pub saved_drafts: Vec<SavedDraft>,
    pub saved_histories: Vec<SavedHistory>,
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
    pub window_width: Option<f32>,
    pub window_height: Option<f32>,
    #[serde(skip)]
    pub load_failed: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            user: String::new(),
            token: String::new(),
            token_error: None,
            last_channel: None,
            theme: "dark".into(),
            font_family: ".SystemUIFont".into(),
            monospace_font: "monospace".into(),
            font_size: 15.0,
            line_height: 1.45,
            message_spacing: 2.0,
            image_cache_limit: 256,
            show_inline_images: true,
            preview_mode: PreviewMode::Shown,
            show_previews: true,
            video_thumbnails: true,
            media_volume: 1.0,
            media_muted: false,
            show_sidebar: true,
            nick_column: true,
            show_timestamps: true,
            use_12h_clock: false,
            show_seconds: false,
            colored_nicks: true,
            status_messages: StatusMessages::Condensed,
            show_user_list: true,
            notifications: true,
            notification_mode: NotificationMode::Mentions,
            notification_overrides: Vec::new(),
            icon_buttons: true,
            show_tray_icon: true,
            enable_global_hotkey: true,
            global_hotkey: "control+shift+l".into(),
            custom_theme: None,
            saved_themes: Vec::new(),
            favorite_commands: Vec::new(),
            command_aliases: HashMap::new(),
            saved_drafts: Vec::new(),
            saved_histories: Vec::new(),
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
            load_failed: false,
        }
    }
}

fn default_media_volume() -> f32 {
    1.0
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
            Ok(text) => match toml::from_str::<Self>(&text) {
                Ok(mut config) => {
                    config.migrate_themes();
                    config.migrate_previews();
                    config.migrate_token();
                    config
                }
                Err(_) => {
                    let _ = fs::copy(&path, path.with_extension("toml.bak"));
                    Self {
                        load_failed: true,
                        ..Default::default()
                    }
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn channel_notification_mode(&self, channel: i64) -> NotificationMode {
        self.notification_overrides
            .iter()
            .rev()
            .find(|entry| entry.channel == channel)
            .map(|entry| entry.mode)
            .unwrap_or(self.notification_mode)
    }

    pub fn channel_notification_override(&self, channel: i64) -> Option<NotificationMode> {
        self.notification_overrides
            .iter()
            .rev()
            .find(|entry| entry.channel == channel)
            .map(|entry| entry.mode)
    }

    pub fn set_channel_notification_mode(&mut self, channel: i64, mode: Option<NotificationMode>) {
        self.notification_overrides
            .retain(|entry| entry.channel != channel);
        if let Some(mode) = mode {
            self.notification_overrides
                .push(NotificationOverride { channel, mode });
        }
    }

    fn migrate_token(&mut self) {
        if self.server.is_empty() || self.user.is_empty() {
            return;
        }
        if !self.token.is_empty() {
            match crate::secure_token::store_token(&self.server, &self.user, &self.token) {
                Ok(()) => self.save(),
                Err(error) => {
                    self.token_error = Some(error);
                    self.save();
                }
            }
            return;
        }
        match crate::secure_token::load_token(&self.server, &self.user) {
            Ok(Some(token)) => self.token = token,
            Ok(None) => {}
            Err(error) => self.token_error = Some(error),
        }
    }

    fn migrate_previews(&mut self) {
        if !self.show_inline_images && self.preview_mode == PreviewMode::Shown {
            self.preview_mode = PreviewMode::Hidden;
        }
    }

    fn migrate_themes(&mut self) {
        let Some(theme) = self.custom_theme.take() else {
            return;
        };
        if !self.saved_themes.iter().any(|saved| saved.id == "custom") {
            self.saved_themes.insert(
                0,
                SavedTheme {
                    id: "custom".into(),
                    name: "Custom".into(),
                    theme,
                },
            );
        }
    }

    pub fn save(&self) {
        if self.load_failed {
            return;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_notification_overrides_shadow_global_mode() {
        let mut config = AppConfig {
            notification_mode: NotificationMode::All,
            ..Default::default()
        };
        assert_eq!(config.channel_notification_mode(7), NotificationMode::All);
        config.set_channel_notification_mode(7, Some(NotificationMode::Mentions));
        assert_eq!(
            config.channel_notification_mode(7),
            NotificationMode::Mentions
        );
        config.set_channel_notification_mode(7, None);
        assert_eq!(config.channel_notification_mode(7), NotificationMode::All);
    }

    #[test]
    fn media_preferences_round_trip() {
        let config = AppConfig {
            media_volume: 0.35,
            media_muted: true,
            ..Default::default()
        };
        let text = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert_eq!(parsed.media_volume, 0.35);
        assert!(parsed.media_muted);
    }

    #[test]
    fn legacy_config_defaults_to_unmuted_media() {
        let config: AppConfig = toml::from_str("notifications = true").unwrap();
        assert_eq!(config.media_volume, 1.0);
        assert!(!config.media_muted);
    }

    #[test]
    fn legacy_config_defaults_to_mention_notifications() {
        let config: AppConfig = toml::from_str("notifications = true").unwrap();
        assert_eq!(config.notification_mode, NotificationMode::Mentions);
        assert!(config.notification_overrides.is_empty());
    }

    #[test]
    fn token_is_not_serialized_to_disk() {
        let config = AppConfig {
            token: "secret-token".into(),
            ..Default::default()
        };
        let text = toml::to_string_pretty(&config).unwrap();
        assert!(!text.contains("secret-token"));
    }

    #[test]
    fn legacy_hidden_images_migrate_to_preview_mode() {
        let mut config = AppConfig {
            show_inline_images: false,
            ..Default::default()
        };
        config.migrate_previews();
        assert_eq!(config.preview_mode, PreviewMode::Hidden);
    }

    #[test]
    fn legacy_custom_theme_migrates_to_saved_theme() {
        let custom = CustomTheme::default();
        let mut config = AppConfig {
            custom_theme: Some(custom.clone()),
            theme: "custom".into(),
            ..Default::default()
        };
        config.migrate_themes();
        assert!(config.custom_theme.is_none());
        assert_eq!(config.saved_themes.len(), 1);
        assert_eq!(config.saved_themes[0].id, "custom");
        assert_eq!(config.saved_themes[0].theme, custom);
    }

    #[test]
    fn invalid_config_keeps_original_and_suppresses_saves() {
        let root = std::env::temp_dir().join(format!("lounge-config-test-{}", std::process::id()));
        let config_dir = root.join("lounge-native");
        fs::create_dir_all(&config_dir).unwrap();
        let path = config_dir.join("config.toml");
        fs::write(&path, "not valid toml = [").unwrap();
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &root);
        }

        let config = AppConfig::load();
        assert!(config.load_failed);
        assert!(config_dir.join("config.toml.bak").exists());

        config.save();
        assert_eq!(fs::read_to_string(&path).unwrap(), "not valid toml = [");

        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        let _ = fs::remove_dir_all(root);
    }
}
