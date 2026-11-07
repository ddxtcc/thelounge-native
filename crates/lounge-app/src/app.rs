use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use chrono::Local;
use gpui::{
    AnyElement, App, Bounds, ClipboardItem, Context, ElementId, Entity, ExternalPaths, Focusable,
    FontWeight, Hsla, InteractiveText, ListAlignment, ListOffset, ListState, MouseButton,
    PathPromptOptions, Pixels, Point, SharedString, StyledText, TextRun, UnderlineStyle, Window,
    div, list, prelude::*, px, relative, rgb,
};
use lounge_core::format::{self, IrcColor};
use lounge_core::proto::{Message, MessageType, ServerEvent, User};
use lounge_core::socket::{Auth, ClientConfig, ClientEvent, ClientHandle};
use lounge_core::state::{ChannelState, ClientState, NetworkState};

use crate::config::{AppConfig, StatusMessages};
use crate::text_input::{InputAction, InputColors, InputRole, TextInput};
use crate::theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Appearance,
    Messages,
    General,
    Server,
    About,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    Login,
    Chat,
}

enum ConnectionStatus {
    Disconnected,
    Connecting,
    Authenticating,
    Online,
    Reconnecting(String),
    Failed(String),
}

impl ConnectionStatus {
    fn label(&self) -> &str {
        match self {
            ConnectionStatus::Disconnected => "Disconnected",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Authenticating => "Authenticating...",
            ConnectionStatus::Online => "Connected",
            ConnectionStatus::Reconnecting(reason) => reason,
            ConnectionStatus::Failed(reason) => reason,
        }
    }

    fn is_online(&self) -> bool {
        matches!(self, ConnectionStatus::Online)
    }
}

pub struct Inputs {
    composer: Entity<TextInput>,
    login_server: Entity<TextInput>,
    login_user: Entity<TextInput>,
    login_password: Entity<TextInput>,
    network_name: Entity<TextInput>,
    network_host: Entity<TextInput>,
    network_port: Entity<TextInput>,
    network_nick: Entity<TextInput>,
    network_username: Entity<TextInput>,
    network_realname: Entity<TextInput>,
    network_password: Entity<TextInput>,
    search: Entity<TextInput>,
    setting_highlights: Entity<TextInput>,
    setting_highlight_exceptions: Entity<TextInput>,
    setting_away_message: Entity<TextInput>,
    emoji_search: Entity<TextInput>,
    font_search: Entity<TextInput>,
    users_filter: Entity<TextInput>,
    palette: Entity<TextInput>,
    password_old: Entity<TextInput>,
    password_new: Entity<TextInput>,
    password_verify: Entity<TextInput>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SelectionPoint {
    message_id: i64,
    index: usize,
}

struct ActiveUpload {
    name: String,
    progress: lounge_core::upload::UploadProgress,
}

#[derive(Clone)]
struct MessageContext {
    channel: String,
    nick: String,
    time: Option<chrono::DateTime<chrono::Utc>>,
    text: String,
}

#[derive(Clone)]
struct TextSelection {
    start: SelectionPoint,
    end: SelectionPoint,
}

impl TextSelection {
    fn range_for(
        &self,
        channel: &ChannelState,
        message_id: i64,
        text: &str,
    ) -> Option<Range<usize>> {
        let len = text.len();
        let position = |id: i64| {
            channel
                .chan
                .messages
                .iter()
                .position(|message| message.id == id)
        };
        let start_position = position(self.start.message_id)?;
        let end_position = position(self.end.message_id)?;
        let current_position = position(message_id)?;
        let (first, second) = if start_position <= end_position {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        };
        let first_position = start_position.min(end_position);
        let second_position = start_position.max(end_position);
        if current_position < first_position || current_position > second_position {
            return None;
        }
        let start = if current_position == first_position {
            first.index
        } else {
            0
        };
        let end = if current_position == second_position {
            second.index
        } else {
            len
        };
        let start = floor_char_boundary(text, start.min(len));
        let end = floor_char_boundary(text, end.min(len));
        Some(start.min(end)..start.max(end))
    }

    fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone)]
struct ReplyTarget {
    msgid: String,
    nick: String,
    text: String,
}

#[derive(Clone)]
struct UserMenu {
    nick: String,
    chan_id: i64,
    position: Point<Pixels>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FontTarget {
    Ui,
    Mono,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PaletteMode {
    Commands,
    Channels,
    Themes,
    Alias,
    Hotkey,
    Users(PaletteUserAction),
}

impl PaletteCommand {
    fn section(self) -> &'static str {
        match self {
            Self::Settings
            | Self::Security
            | Self::SettingsHighlights
            | Self::SettingsHighlightExceptions
            | Self::SettingsAwayMessage
            | Self::AddAlias => "Settings",
            Self::Shortcuts => "Help",
            Self::SettingsTab(_)
            | Self::ToggleTray
            | Self::ToggleGlobalHotkey
            | Self::SetGlobalHotkey => "Settings",
            Self::SwitchChannel
            | Self::NextChannel
            | Self::PreviousChannel
            | Self::Join
            | Self::LoadOlder
            | Self::JumpToBottom => "Navigation",
            Self::Search | Self::Mentions | Self::Emoji => "Messages",
            Self::UploadFile
            | Self::ClearComposer
            | Self::PasteImage
            | Self::CancelReply
            | Self::CopySelection => "Composer",
            Self::Theme
            | Self::ChooseUiFont
            | Self::ChooseMonoFont
            | Self::ImportTheme
            | Self::ExportTheme
            | Self::ToggleSidebar
            | Self::ToggleUsers
            | Self::ToggleInlineImages
            | Self::TogglePreviews
            | Self::ToggleTimestamps
            | Self::Toggle24Hour
            | Self::ToggleSeconds
            | Self::ToggleColoredNicks
            | Self::ToggleNotifications
            | Self::ToggleIconButtons
            | Self::ToggleAlignedText
            | Self::CycleStatusMessages
            | Self::FontSizeUp
            | Self::FontSizeDown
            | Self::LineHeightUp
            | Self::LineHeightDown
            | Self::MessageSpacingUp
            | Self::MessageSpacingDown => "Appearance",
            Self::MoveChannelUp
            | Self::MoveChannelDown
            | Self::LeaveChannel
            | Self::ToggleMute
            | Self::ClearChannelHistory
            | Self::EditTopic => "Channels",
            Self::AddNetwork
            | Self::EditNetwork
            | Self::ReconnectNetwork
            | Self::DisconnectNetwork
            | Self::RemoveNetwork
            | Self::MoveNetworkUp
            | Self::MoveNetworkDown => "Networks",
            Self::UserAction(_) => "Users",
            Self::SignOut => "Account",
        }
    }
}

#[derive(Clone)]
enum PaletteEntry {
    Command(PaletteCommand),
    Recent(PaletteCommand),
    Favorite(PaletteCommand),
    Channel(i64),
    Theme(String, String),
    User {
        action: PaletteUserAction,
        nick: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PaletteCommand {
    Settings,
    Security,
    SwitchChannel,
    NextChannel,
    PreviousChannel,
    Join,
    UploadFile,
    Search,
    Mentions,
    Emoji,
    Theme,
    ChooseUiFont,
    ChooseMonoFont,
    ImportTheme,
    ExportTheme,
    ToggleSidebar,
    ToggleUsers,
    ToggleInlineImages,
    TogglePreviews,
    ToggleTimestamps,
    Toggle24Hour,
    ToggleSeconds,
    ToggleColoredNicks,
    ToggleNotifications,
    ToggleIconButtons,
    ToggleAlignedText,
    CycleStatusMessages,
    FontSizeUp,
    FontSizeDown,
    LineHeightUp,
    LineHeightDown,
    MessageSpacingUp,
    MessageSpacingDown,
    SettingsHighlights,
    SettingsHighlightExceptions,
    SettingsAwayMessage,
    MoveChannelUp,
    MoveChannelDown,
    AddNetwork,
    EditNetwork,
    ReconnectNetwork,
    DisconnectNetwork,
    RemoveNetwork,
    MoveNetworkUp,
    MoveNetworkDown,
    LeaveChannel,
    ToggleMute,
    ClearChannelHistory,
    EditTopic,
    LoadOlder,
    JumpToBottom,
    ClearComposer,
    PasteImage,
    CancelReply,
    CopySelection,
    SignOut,
    AddAlias,
    Shortcuts,
    SettingsTab(SettingsTab),
    ToggleTray,
    ToggleGlobalHotkey,
    SetGlobalHotkey,
    UserAction(PaletteUserAction),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PaletteUserAction {
    Mention,
    Query,
    Whois,
    Op,
    Deop,
    Voice,
    Devoice,
    Kick,
    Ban,
}

impl PaletteUserAction {
    fn command(self) -> &'static str {
        match self {
            Self::Mention => "mention",
            Self::Query => "query",
            Self::Whois => "whois",
            Self::Op => "op",
            Self::Deop => "deop",
            Self::Voice => "voice",
            Self::Devoice => "devoice",
            Self::Kick => "kick",
            Self::Ban => "ban",
        }
    }

    fn prompt(self) -> &'static str {
        match self {
            Self::Mention => "Choose a user to mention...",
            Self::Query => "Choose a user to query...",
            Self::Whois => "Choose a user to inspect...",
            Self::Op => "Choose a user to op...",
            Self::Deop => "Choose a user to deop...",
            Self::Voice => "Choose a user to voice...",
            Self::Devoice => "Choose a user to devoice...",
            Self::Kick => "Choose a user to kick...",
            Self::Ban => "Choose a user to ban...",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Mention => "Mention user",
            Self::Query => "Open query",
            Self::Whois => "Whois user",
            Self::Op => "Give op",
            Self::Deop => "Remove op",
            Self::Voice => "Give voice",
            Self::Devoice => "Remove voice",
            Self::Kick => "Kick user",
            Self::Ban => "Ban user",
        }
    }

    fn destructive(self) -> bool {
        matches!(self, Self::Kick | Self::Ban)
    }
}

#[derive(Clone)]
struct ChannelMenu {
    chan_id: i64,
    network_uuid: String,
    name: String,
    is_query: bool,
    muted: bool,
    position: Point<Pixels>,
}

#[derive(Clone)]
struct MessageMenu {
    position: Point<Pixels>,
    channel_id: i64,
    message: Message,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKind {
    Network,
    Channel,
}

#[derive(Clone)]
struct SortDrag {
    kind: SortKind,
    network_uuid: String,
    channel_id: Option<i64>,
    label: String,
}

enum SortTarget {
    Network(String),
    Channel {
        network_uuid: String,
        channel_id: i64,
    },
}

struct DragPreview {
    label: SharedString,
    position: Point<Pixels>,
    color: Hsla,
    text_color: Hsla,
}

impl Render for DragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .pl(self.position.x - px(60.))
            .pt(self.position.y - px(16.))
            .child(
                div()
                    .max_w(px(180.))
                    .truncate()
                    .px_3()
                    .py_1()
                    .rounded(px(5.))
                    .bg(self.color)
                    .text_color(self.text_color)
                    .text_size(px(12.))
                    .shadow_md()
                    .child(self.label.clone()),
            )
    }
}

#[derive(Clone)]
enum ClickTarget {
    Url(String),
    Nick(String),
}

struct Toast {
    text: String,
    until: Instant,
}

pub struct LoungeApp {
    config: AppConfig,
    theme: Theme,
    state: ClientState,
    conn: Option<ClientHandle>,
    conn_generation: u64,
    status: ConnectionStatus,
    route: Route,
    login_error: Option<String>,
    inputs: Inputs,
    input_revisions: HashMap<InputRole, u64>,
    messages_list: ListState,
    messages_list_keys: Vec<String>,
    messages_at_bottom: bool,
    messages_bounds: Option<Bounds<Pixels>>,
    messages_available: Pixels,
    message_rows_cache: Option<message_list::MessageRowsCache>,
    scroll_to_bottom_pending: bool,
    image_cache: Entity<gpui::RetainAllImageCache>,
    image_cache_current: RefCell<HashSet<String>>,
    image_cache_touched: RefCell<VecDeque<String>>,
    pending_focus_message: Option<i64>,
    focused_message: Option<i64>,
    collapsed_networks: HashSet<String>,
    expanded_condensed: HashSet<i64>,
    narrow: bool,
    sidebar_overlay: bool,
    users_overlay: bool,
    lightbox: Option<String>,
    lightbox_index: Option<usize>,
    lightbox_zoom: f32,
    #[cfg(feature = "media-video")]
    media_video: Option<(String, gpui_video_player::Video)>,
    #[cfg(feature = "media-video")]
    media_error: Option<String>,
    lightbox_pan: Point<Pixels>,
    lightbox_drag_last: Option<Point<Pixels>>,
    drafts: HashMap<i64, String>,
    histories: HashMap<i64, Vec<String>>,
    history_index: usize,
    history_draft: String,
    reply_to: Option<ReplyTarget>,
    typing_generation: u64,
    typing_active: bool,
    last_typing_sent: Option<Instant>,
    toast: Option<Toast>,
    notification_action_tx: futures::channel::mpsc::UnboundedSender<i64>,
    window_title: String,
    hotkey_settings_return: bool,
    #[cfg(target_os = "linux")]
    tray_handle: Option<ksni::blocking::Handle<tray::LoungeTray>>,
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    hotkey_manager: Option<global_hotkey::GlobalHotKeyManager>,
    #[cfg(target_os = "windows")]
    tray_handle: Option<tray_windows::TrayHandle>,
    settings_open: bool,
    settings_tab: SettingsTab,
    network_modal_open: bool,
    search_open: bool,
    shortcuts_open: bool,
    search_selection: usize,
    search_scroll: gpui::ScrollHandle,
    mentions_open: bool,
    mentions_selection: usize,
    mentions_scroll: gpui::ScrollHandle,
    seen_mentions: HashSet<i64>,
    message_context: Option<MessageContext>,
    pending_context: Option<MessageContext>,
    emoji_selection: usize,
    emoji_scroll: gpui::ScrollHandle,
    network_tls: bool,
    network_defaults_applied: bool,
    pending_focus: bool,
    last_active_channel: Option<i64>,
    last_message_id: i64,
    last_known_revision: u64,
    pending_uploads: Vec<PathBuf>,
    uploads_in_flight: usize,
    active_upload: Option<ActiveUpload>,
    window_active: bool,
    focus_request: Option<InputRole>,
    focus_root_request: bool,
    emoji_picker_open: bool,
    security_open: bool,
    network_menu: Option<String>,
    network_edit_uuid: Option<String>,
    network_edit_data: Option<serde_json::Map<String, serde_json::Value>>,
    network_edit_loading: bool,
    network_has_sts: bool,
    user_menu: Option<UserMenu>,
    channel_menu: Option<ChannelMenu>,
    message_menu: Option<MessageMenu>,
    text_selection: Option<TextSelection>,
    hovered_text: Option<(i64, usize)>,
    selecting_text: bool,
    confirm_remove_network: Option<String>,
    root_focus: gpui::FocusHandle,
    last_overlay_open: bool,
    fonts: Vec<String>,
    fonts_loading: bool,
    fonts_loaded: bool,
    font_spinner: usize,
    font_picker: Option<FontTarget>,
    palette: Option<PaletteMode>,
    palette_selection: usize,
    palette_scroll: gpui::ScrollHandle,
    recent_commands: Vec<PaletteCommand>,
    pending_channel_switch: Option<i64>,
    pending_show_window: bool,
    window_save_generation: u64,
    scroll_after_more: bool,
}

mod actions;
#[cfg(target_os = "linux")]
mod badge;
mod commands;
mod connection;
mod desktop;
#[cfg(feature = "media-video")]
mod media;
mod menus;
mod message_list;
mod message_text;
mod messages;
mod navigation;
mod overlays;
mod palette;
mod render;
mod render_impl;
mod settings;
mod sidebar;
mod special;
mod tooltip;
#[cfg(target_os = "linux")]
mod tray;
#[cfg(target_os = "windows")]
mod tray_windows;
mod uploads;

impl LoungeApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let config = AppConfig::load();
        let theme = Theme::from_config(&config.theme, config.custom_theme.as_ref());
        let root_focus = cx.focus_handle();
        let colors = theme.input_colors();
        let font = config.font_size;
        let line = (font * config.line_height).max(20.0);

        let mut next_tab_index = 0;
        let mut make =
            |placeholder: &'static str, multiline: bool, masked: bool, cx: &mut Context<Self>| {
                next_tab_index += 10;
                cx.new(|cx| {
                    let mut input =
                        TextInput::new(placeholder, colors.clone(), multiline, masked, cx);
                    input.set_text_size(font, line);
                    input.with_tab_index(next_tab_index)
                })
            };

        let composer = make("Message...", true, false, cx);
        let login_server = make("https://lounge.example.com", false, false, cx);
        let login_user = make("Username", false, false, cx);
        let login_password = make("Password", false, true, cx);
        let network_name = make("Network name", false, false, cx);
        let network_host = make("irc.libera.chat", false, false, cx);
        let network_port = make("6697", false, false, cx);
        let network_nick = make("Nickname", false, false, cx);
        let network_username = make("Username", false, false, cx);
        let network_realname = make("Real name", false, false, cx);
        let network_password = make("Server password", false, true, cx);
        let search = make("Search this channel...", false, false, cx);
        let setting_highlights = make("Comma-separated highlight words", false, false, cx);
        let setting_highlight_exceptions = make("Comma-separated exceptions", false, false, cx);
        let setting_away_message = make("Away message", false, false, cx);
        let emoji_search = make("Search emoji...", false, false, cx);
        let font_search = make("Search installed fonts...", false, false, cx);
        let users_filter = make("Filter users...", false, false, cx);
        let palette = make("Type a command...", false, false, cx);
        let password_old = make("Current password", false, true, cx);
        let password_new = make("New password", false, true, cx);
        let password_verify = make("Verify new password", false, true, cx);

        let inputs = Inputs {
            composer: composer.clone(),
            login_server: login_server.clone(),
            login_user: login_user.clone(),
            login_password: login_password.clone(),
            network_name: network_name.clone(),
            network_host: network_host.clone(),
            network_port: network_port.clone(),
            network_nick: network_nick.clone(),
            network_username: network_username.clone(),
            network_realname: network_realname.clone(),
            network_password: network_password.clone(),
            search: search.clone(),
            setting_highlights: setting_highlights.clone(),
            setting_highlight_exceptions: setting_highlight_exceptions.clone(),
            setting_away_message: setting_away_message.clone(),
            emoji_search: emoji_search.clone(),
            font_search: font_search.clone(),
            users_filter: users_filter.clone(),
            palette: palette.clone(),
            password_old: password_old.clone(),
            password_new: password_new.clone(),
            password_verify: password_verify.clone(),
        };

        let pairs = [
            (InputRole::Composer, composer),
            (InputRole::LoginServer, login_server),
            (InputRole::LoginUser, login_user),
            (InputRole::LoginPassword, login_password),
            (InputRole::NetworkName, network_name),
            (InputRole::NetworkHost, network_host),
            (InputRole::NetworkPort, network_port),
            (InputRole::NetworkNick, network_nick),
            (InputRole::NetworkUsername, network_username),
            (InputRole::NetworkRealname, network_realname),
            (InputRole::NetworkPassword, network_password),
            (InputRole::Search, search),
            (InputRole::SettingHighlights, setting_highlights),
            (
                InputRole::SettingHighlightExceptions,
                setting_highlight_exceptions,
            ),
            (InputRole::SettingAwayMessage, setting_away_message),
            (InputRole::EmojiSearch, emoji_search),
            (InputRole::FontSearch, font_search),
            (InputRole::UsersFilter, users_filter),
            (InputRole::Palette, palette),
            (InputRole::PasswordOld, password_old),
            (InputRole::PasswordNew, password_new),
            (InputRole::PasswordVerify, password_verify),
        ];
        for (role, input) in pairs {
            cx.observe(&input, move |this, input, cx| {
                this.on_input_event(role, input, cx);
            })
            .detach();
        }

        let drafts: HashMap<i64, String> = config
            .saved_drafts
            .iter()
            .map(|draft| (draft.channel, draft.text.clone()))
            .collect();
        let histories: HashMap<i64, Vec<String>> = config
            .saved_histories
            .iter()
            .map(|history| (history.channel, history.entries.clone()))
            .collect();

        let (notification_action_tx, notification_action_rx) = futures::channel::mpsc::unbounded();
        #[cfg(target_os = "linux")]
        let (tray_handle, tray_action_rx) = match config.show_tray_icon.then(tray::spawn).flatten()
        {
            Some((handle, receiver)) => (Some(handle), Some(receiver)),
            None => (None, None),
        };
        #[cfg(target_os = "windows")]
        let (tray_handle, tray_action_rx) =
            match config.show_tray_icon.then(tray_windows::spawn).flatten() {
                Some((handle, receiver)) => (Some(handle), Some(receiver)),
                None => (None, None),
            };
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        let hotkey_manager = {
            use global_hotkey::{
                GlobalHotKeyManager,
                hotkey::{Code, HotKey, Modifiers},
            };
            let manager = config
                .enable_global_hotkey
                .then(GlobalHotKeyManager::new)
                .and_then(Result::ok);
            if let Some(manager) = &manager {
                let hotkey = config
                    .global_hotkey
                    .parse::<HotKey>()
                    .ok()
                    .filter(|hotkey| !hotkey.mods.is_empty())
                    .unwrap_or_else(|| {
                        HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyL)
                    });
                let _ = manager.register(hotkey);
            }
            manager
        };
        let mut app = Self {
            config,
            theme,
            state: ClientState::new(),
            conn: None,
            conn_generation: 0,
            status: ConnectionStatus::Disconnected,
            route: Route::Login,
            login_error: None,
            inputs,
            input_revisions: HashMap::new(),
            messages_list: ListState::new(0, ListAlignment::Bottom, px(600.)),
            messages_list_keys: Vec::new(),
            messages_at_bottom: true,
            messages_bounds: None,
            messages_available: px(0.),
            message_rows_cache: None,
            scroll_to_bottom_pending: false,
            image_cache: gpui::RetainAllImageCache::new(cx),
            image_cache_current: RefCell::new(HashSet::new()),
            image_cache_touched: RefCell::new(VecDeque::new()),
            pending_focus_message: None,
            focused_message: None,
            collapsed_networks: HashSet::new(),
            expanded_condensed: HashSet::new(),
            narrow: false,
            sidebar_overlay: false,
            users_overlay: false,
            lightbox: None,
            lightbox_index: None,
            lightbox_zoom: 1.0,
            #[cfg(feature = "media-video")]
            media_video: None,
            #[cfg(feature = "media-video")]
            media_error: None,
            lightbox_pan: Point::default(),
            lightbox_drag_last: None,
            drafts,
            histories,
            history_index: 0,
            history_draft: String::new(),
            reply_to: None,
            typing_generation: 0,
            typing_active: false,
            last_typing_sent: None,
            toast: None,
            notification_action_tx,
            window_title: "Lounge Native".to_string(),
            hotkey_settings_return: false,
            #[cfg(target_os = "linux")]
            tray_handle,
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            hotkey_manager,
            #[cfg(target_os = "windows")]
            tray_handle,
            settings_open: false,
            settings_tab: SettingsTab::Appearance,
            network_modal_open: false,
            search_open: false,
            shortcuts_open: false,
            search_selection: 0,
            search_scroll: gpui::ScrollHandle::new(),
            mentions_open: false,
            mentions_selection: 0,
            mentions_scroll: gpui::ScrollHandle::new(),
            seen_mentions: HashSet::new(),
            message_context: None,
            pending_context: None,
            emoji_selection: 0,
            emoji_scroll: gpui::ScrollHandle::new(),
            network_tls: true,
            network_defaults_applied: false,
            pending_focus: true,
            last_active_channel: None,
            last_message_id: 0,
            last_known_revision: 0,
            pending_uploads: Vec::new(),
            uploads_in_flight: 0,
            active_upload: None,
            window_active: true,
            focus_request: None,
            focus_root_request: false,
            emoji_picker_open: false,
            security_open: false,
            network_menu: None,
            network_edit_uuid: None,
            network_edit_data: None,
            network_edit_loading: false,
            network_has_sts: false,
            user_menu: None,
            channel_menu: None,
            message_menu: None,
            text_selection: None,
            hovered_text: None,
            selecting_text: false,
            confirm_remove_network: None,
            root_focus,
            last_overlay_open: false,
            fonts: Vec::new(),
            fonts_loading: false,
            fonts_loaded: false,
            font_spinner: 0,
            font_picker: None,
            palette: None,
            palette_selection: 0,
            palette_scroll: gpui::ScrollHandle::new(),
            recent_commands: Vec::new(),
            pending_channel_switch: None,
            pending_show_window: false,
            window_save_generation: 0,
            scroll_after_more: false,
        };
        let list = app.messages_list.clone();
        let weak = cx.entity().downgrade();
        list.set_scroll_handler(move |_, _, cx| {
            let weak = weak.clone();
            cx.defer(move |cx| {
                let _ = weak.update(cx, |this, _| {
                    let top = this.messages_list.logical_scroll_top();
                    this.messages_at_bottom = top.item_ix + 3 >= this.messages_list.item_count();
                });
            });
        });
        app.watch_notifications(notification_action_rx, cx);
        #[cfg(target_os = "linux")]
        if let Some(receiver) = tray_action_rx {
            app.watch_tray(receiver, cx);
        }
        #[cfg(target_os = "windows")]
        if let Some(receiver) = tray_action_rx {
            app.watch_tray(receiver, cx);
        }
        if app.config.load_failed {
            app.toast("Config could not be parsed; using defaults and keeping a backup.");
        }
        app.prefill_login(cx);

        if !app.config.server.is_empty() && !app.config.token.is_empty() {
            let auth = Auth::Token {
                user: app.config.user.clone(),
                token: app.config.token.clone(),
            };
            app.start_connection(auth, app.config.server.clone(), cx);
        }

        app
    }
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel_with_ids(ids: &[i64]) -> ChannelState {
        ChannelState {
            chan: lounge_core::proto::Channel {
                messages: ids
                    .iter()
                    .map(|id| Message {
                        id: *id,
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn text_selection_ranges_forward_across_messages() {
        let channel = channel_with_ids(&[10, 20, 30]);
        let selection = TextSelection {
            start: SelectionPoint {
                message_id: 10,
                index: 2,
            },
            end: SelectionPoint {
                message_id: 30,
                index: 4,
            },
        };
        assert_eq!(
            selection.range_for(&channel, 10, &"x".repeat(12)),
            Some(2..12)
        );
        assert_eq!(
            selection.range_for(&channel, 20, &"x".repeat(12)),
            Some(0..12)
        );
        assert_eq!(
            selection.range_for(&channel, 30, &"x".repeat(12)),
            Some(0..4)
        );
        assert_eq!(selection.range_for(&channel, 99, &"x".repeat(12)), None);
    }

    #[test]
    fn text_selection_normalizes_backward_drag() {
        let channel = channel_with_ids(&[10, 20, 30]);
        let selection = TextSelection {
            start: SelectionPoint {
                message_id: 30,
                index: 4,
            },
            end: SelectionPoint {
                message_id: 10,
                index: 2,
            },
        };
        assert_eq!(
            selection.range_for(&channel, 10, &"x".repeat(12)),
            Some(2..12)
        );
        assert_eq!(
            selection.range_for(&channel, 20, &"x".repeat(12)),
            Some(0..12)
        );
        assert_eq!(
            selection.range_for(&channel, 30, &"x".repeat(12)),
            Some(0..4)
        );
    }

    #[test]
    fn text_selection_clamps_indexes_to_message_length() {
        let channel = channel_with_ids(&[10]);
        let selection = TextSelection {
            start: SelectionPoint {
                message_id: 10,
                index: 50,
            },
            end: SelectionPoint {
                message_id: 10,
                index: 80,
            },
        };
        assert_eq!(
            selection.range_for(&channel, 10, &"x".repeat(12)),
            Some(12..12)
        );
    }

    #[test]
    fn text_selection_clamps_to_char_boundaries() {
        let channel = channel_with_ids(&[10]);
        let selection = TextSelection {
            start: SelectionPoint {
                message_id: 10,
                index: 1,
            },
            end: SelectionPoint {
                message_id: 10,
                index: 3,
            },
        };
        assert_eq!(selection.range_for(&channel, 10, "a日本"), Some(1..1));
    }
}
