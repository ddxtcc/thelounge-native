use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use chrono::Local;
use gpui::{
    AnyElement, App, Context, ElementId, Entity, ExternalPaths, Focusable, FontWeight, Hsla,
    InteractiveText, MouseButton, PathPromptOptions, Pixels, Point, SharedString, StyledText,
    TextRun, UnderlineStyle, Window, div, prelude::*, px, relative, rgb,
};
use lounge_core::format::{self, IrcColor};
use lounge_core::proto::{Message, MessageType, ServerEvent, User};
use lounge_core::socket::{Auth, ClientConfig, ClientEvent, ClientHandle};
use lounge_core::state::{ChannelState, ClientState, NetworkState};

use crate::config::{AppConfig, StatusMessages};
use crate::text_input::{InputAction, InputColors, InputRole, TextInput};
use crate::theme::Theme;

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
    password_old: Entity<TextInput>,
    password_new: Entity<TextInput>,
    password_verify: Entity<TextInput>,
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FontTarget {
    Ui,
    Mono,
}

#[derive(Clone)]
struct ChannelMenu {
    chan_id: i64,
    network_uuid: String,
    name: String,
    is_query: bool,
    muted: bool,
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
    messages_scroll: gpui::ScrollHandle,
    image_cache: Entity<gpui::RetainAllImageCache>,
    collapsed_networks: HashSet<String>,
    expanded_condensed: HashSet<i64>,
    narrow: bool,
    sidebar_overlay: bool,
    users_overlay: bool,
    lightbox: Option<String>,
    drafts: HashMap<i64, String>,
    histories: HashMap<i64, Vec<String>>,
    history_index: usize,
    history_draft: String,
    reply_to: Option<ReplyTarget>,
    typing_generation: u64,
    typing_active: bool,
    last_typing_sent: Option<Instant>,
    toast: Option<Toast>,
    settings_open: bool,
    network_modal_open: bool,
    search_open: bool,
    mentions_open: bool,
    network_tls: bool,
    network_defaults_applied: bool,
    pending_focus: bool,
    last_active_channel: Option<i64>,
    last_message_id: i64,
    last_known_revision: u64,
    pending_uploads: Vec<PathBuf>,
    uploads_in_flight: usize,
    window_active: bool,
    focus_request: Option<InputRole>,
    emoji_picker_open: bool,
    security_open: bool,
    network_menu: Option<String>,
    network_edit_uuid: Option<String>,
    network_has_sts: bool,
    user_menu: Option<UserMenu>,
    channel_menu: Option<ChannelMenu>,
    confirm_remove_network: Option<String>,
    root_focus: gpui::FocusHandle,
    last_overlay_open: bool,
    fonts: Vec<String>,
    fonts_loading: bool,
    fonts_loaded: bool,
    font_spinner: usize,
    font_picker: Option<FontTarget>,
    scroll_generation: u64,
    scroll_after_more: bool,
}

mod actions;
mod connection;
mod menus;
mod message_text;
mod messages;
mod overlays;
mod render;
mod render_impl;
mod settings;
mod uploads;

impl LoungeApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let config = AppConfig::load();
        let theme = Theme::from_mode(&config.theme);
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

        let show_users = config.show_user_list;
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
            messages_scroll: gpui::ScrollHandle::new(),
            image_cache: gpui::RetainAllImageCache::new(cx),
            collapsed_networks: HashSet::new(),
            expanded_condensed: HashSet::new(),
            narrow: false,
            sidebar_overlay: false,
            users_overlay: false,
            lightbox: None,
            drafts: HashMap::new(),
            histories: HashMap::new(),
            history_index: 0,
            history_draft: String::new(),
            reply_to: None,
            typing_generation: 0,
            typing_active: false,
            last_typing_sent: None,
            toast: None,
            settings_open: false,
            network_modal_open: false,
            search_open: false,
            mentions_open: false,
            network_tls: true,
            network_defaults_applied: false,
            pending_focus: true,
            last_active_channel: None,
            last_message_id: 0,
            last_known_revision: 0,
            pending_uploads: Vec::new(),
            uploads_in_flight: 0,
            window_active: true,
            focus_request: None,
            emoji_picker_open: false,
            security_open: false,
            network_menu: None,
            network_edit_uuid: None,
            network_has_sts: false,
            user_menu: None,
            channel_menu: None,
            confirm_remove_network: None,
            root_focus,
            last_overlay_open: false,
            fonts: Vec::new(),
            fonts_loading: false,
            fonts_loaded: false,
            font_spinner: 0,
            font_picker: None,
            scroll_generation: 0,
            scroll_after_more: false,
        };
        let _ = show_users;
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
