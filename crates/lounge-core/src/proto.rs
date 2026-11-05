use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ChanType {
    Channel,
    Lobby,
    Query,
    Special,
    #[default]
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(try_from = "i64", into = "i64")]
pub enum ChanState {
    #[default]
    Parted,
    Joined,
}

impl From<ChanState> for i64 {
    fn from(value: ChanState) -> Self {
        match value {
            ChanState::Parted => 0,
            ChanState::Joined => 1,
        }
    }
}

impl TryFrom<i64> for ChanState {
    type Error = String;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Parted),
            1 => Ok(Self::Joined),
            other => Err(format!("invalid channel state {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Unhandled,
    Action,
    Away,
    Back,
    Error,
    Invite,
    Join,
    Kick,
    Login,
    Logout,
    Message,
    Mode,
    ModeChannel,
    ModeUser,
    MonospaceBlock,
    Nick,
    Notice,
    Part,
    Quit,
    Ctcp,
    CtcpRequest,
    Chghost,
    Topic,
    TopicSetBy,
    Whois,
    Raw,
    Plugin,
    Wallops,
    Warn,
    Note,
    #[default]
    #[serde(other)]
    Unknown,
}

impl MessageType {
    pub fn is_condensable(self) -> bool {
        matches!(
            self,
            Self::Away
                | Self::Back
                | Self::Chghost
                | Self::Join
                | Self::Kick
                | Self::Mode
                | Self::Nick
                | Self::Part
                | Self::Quit
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct User {
    #[serde(default)]
    pub modes: Vec<String>,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub away: String,
    #[serde(default)]
    pub nick: String,
    #[serde(default, rename = "lastMessage")]
    pub last_message: i64,
    #[serde(default, rename = "isBot")]
    pub is_bot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserInMessage {
    #[serde(default)]
    pub modes: Vec<String>,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub away: String,
    #[serde(default)]
    pub nick: String,
    #[serde(default, rename = "lastMessage")]
    pub last_message: i64,
    #[serde(default, rename = "isBot")]
    pub is_bot: bool,
}

impl UserInMessage {
    pub fn nick(&self) -> &str {
        &self.nick
    }

    pub fn mode_prefix<'a>(&self, server_options: Option<&'a ServerOptions>) -> Option<&'a str> {
        if self.mode.is_empty() {
            return None;
        }

        server_options
            .and_then(|opts| opts.prefix.mode_to_symbol.get(&self.mode))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinkPreview {
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub head: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub thumb: String,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub link: String,
    #[serde(default)]
    pub shown: Option<bool>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub media: Option<String>,
    #[serde(default, rename = "mediaType")]
    pub media_type: Option<String>,
    #[serde(default, rename = "maxSize")]
    pub max_size: Option<u64>,
    #[serde(default, rename = "thumbActualUrl")]
    pub thumb_actual_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub from: Option<UserInMessage>,
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub msgid: Option<String>,
    #[serde(default)]
    pub previews: Vec<LinkPreview>,
    #[serde(default)]
    pub text: String,
    #[serde(default, rename = "type")]
    pub kind: MessageType,
    #[serde(default, rename = "self")]
    pub is_self: bool,
    #[serde(default)]
    pub time: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub hostmask: Option<String>,
    #[serde(default)]
    pub target: Option<UserInMessage>,
    #[serde(default)]
    pub new_nick: Option<String>,
    #[serde(default)]
    pub highlight: bool,
    #[serde(default)]
    pub show_in_active: bool,
    #[serde(default)]
    pub new_ident: Option<String>,
    #[serde(default)]
    pub new_host: Option<String>,
    #[serde(default)]
    pub ctcp_message: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub invited_you: bool,
    #[serde(default)]
    pub gecos: Option<String>,
    #[serde(default)]
    pub account: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub nick: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub raw_modes: Option<Value>,
    #[serde(default)]
    pub when: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub whois: Option<Value>,
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub statusmsg_group: Option<String>,
    #[serde(default)]
    pub params: Vec<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub reply_to_nick: Option<String>,
    #[serde(default)]
    pub reply_to_text: Option<String>,
    #[serde(default)]
    pub multiline: bool,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            from: None,
            id: 0,
            msgid: None,
            previews: Vec::new(),
            text: String::new(),
            kind: MessageType::Message,
            is_self: false,
            time: None,
            hostmask: None,
            target: None,
            new_nick: None,
            highlight: false,
            show_in_active: false,
            new_ident: None,
            new_host: None,
            ctcp_message: None,
            command: None,
            invited_you: false,
            gecos: None,
            account: false,
            error: None,
            nick: None,
            channel: None,
            reason: None,
            raw_modes: None,
            when: None,
            whois: None,
            users: Vec::new(),
            statusmsg_group: None,
            params: Vec::new(),
            reply_to: None,
            reply_to_nick: None,
            reply_to_text: None,
            multiline: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Channel {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default, rename = "totalMessages")]
    pub total_messages: usize,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default, rename = "firstUnread")]
    pub first_unread: i64,
    #[serde(default)]
    pub unread: u32,
    #[serde(default)]
    pub highlight: u32,
    #[serde(default)]
    pub muted: bool,
    #[serde(default, rename = "type")]
    pub kind: ChanType,
    #[serde(default)]
    pub state: ChanState,
    #[serde(default, rename = "isOnline")]
    pub is_online: Option<bool>,
    #[serde(default, rename = "userAway")]
    pub user_away: Option<String>,
    #[serde(default)]
    pub special: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
    #[serde(default)]
    pub closed: Option<bool>,
    #[serde(default, rename = "num_users")]
    pub num_users: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkStatus {
    #[serde(default)]
    pub connected: bool,
    #[serde(default)]
    pub secure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrefixObject {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Prefix {
    #[serde(default)]
    pub prefix: Vec<PrefixObject>,
    #[serde(default, rename = "modeToSymbol")]
    pub mode_to_symbol: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerOptions {
    #[serde(default, rename = "CHANTYPES")]
    pub chantypes: Vec<String>,
    #[serde(default, rename = "PREFIX")]
    pub prefix: Prefix,
    #[serde(default, rename = "NETWORK")]
    pub network: String,
    #[serde(default, rename = "supportsReply")]
    pub supports_reply: bool,
    #[serde(default, rename = "MONITOR")]
    pub monitor: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Network {
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub nick: String,
    #[serde(default, rename = "serverOptions")]
    pub server_options: Option<ServerOptions>,
    #[serde(default)]
    pub status: NetworkStatus,
    #[serde(default)]
    pub channels: Vec<Channel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitData {
    #[serde(default)]
    pub active: i64,
    #[serde(default)]
    pub networks: Vec<Network>,
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigTheme {
    #[serde(default, rename = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "themeColor")]
    pub theme_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkDefaults {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub tls: bool,
    #[serde(default, rename = "rejectUnauthorized")]
    pub reject_unauthorized: bool,
    #[serde(default)]
    pub nick: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub realname: String,
    #[serde(default)]
    pub join: String,
    #[serde(default, rename = "leaveMessage")]
    pub leave_message: String,
    #[serde(default)]
    pub sasl: String,
    #[serde(default, rename = "saslAccount")]
    pub sasl_account: String,
    #[serde(default, rename = "saslPassword")]
    pub sasl_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Configuration {
    #[serde(default)]
    pub public: bool,
    #[serde(default, rename = "useHexIp")]
    pub use_hex_ip: bool,
    #[serde(default)]
    pub prefetch: bool,
    #[serde(default, rename = "fileUpload")]
    pub file_upload: bool,
    #[serde(default, rename = "ldapEnabled")]
    pub ldap_enabled: bool,
    #[serde(default, rename = "isUpdateAvailable")]
    pub is_update_available: bool,
    #[serde(default, rename = "applicationServerKey")]
    pub application_server_key: String,
    #[serde(default)]
    pub version: String,
    #[serde(default, rename = "gitCommit")]
    pub git_commit: Option<String>,
    #[serde(default)]
    pub themes: Vec<ConfigTheme>,
    #[serde(default, rename = "defaultTheme")]
    pub default_theme: String,
    #[serde(default, rename = "fileUploadMaxFileSize")]
    pub file_upload_max_file_size: Option<u64>,
    #[serde(default, rename = "lockNetwork")]
    pub lock_network: bool,
    #[serde(default)]
    pub defaults: Option<NetworkDefaults>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    #[serde(default)]
    pub current: bool,
    #[serde(default)]
    pub active: i64,
    #[serde(default, rename = "lastUse")]
    pub last_use: i64,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Mention {
    #[serde(default, rename = "chanId")]
    pub chan_id: i64,
    #[serde(default, rename = "msgId")]
    pub msg_id: i64,
    #[serde(default, rename = "type")]
    pub kind: MessageType,
    #[serde(default)]
    pub time: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub from: UserInMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchQuery {
    #[serde(default, rename = "searchTerm")]
    pub search_term: String,
    #[serde(default, rename = "networkUuid")]
    pub network_uuid: String,
    #[serde(default, rename = "channelName")]
    pub channel_name: String,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResponse {
    #[serde(flatten)]
    pub query: SearchQuery,
    #[serde(default)]
    pub results: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInfo {
    #[serde(default)]
    pub uuid: String,
    #[serde(flatten)]
    pub data: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingEvent {
    pub network: String,
    pub chan: i64,
    pub nick: String,
    pub status: TypingStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TypingStatus {
    #[default]
    Active,
    Paused,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkStatusEvent {
    pub network: String,
    #[serde(default)]
    pub connected: bool,
    #[serde(default)]
    pub secure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MsgEvent {
    pub msg: Message,
    pub chan: i64,
    #[serde(default)]
    pub highlight: Option<u32>,
    #[serde(default)]
    pub unread: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessagePreviewEvent {
    pub id: i64,
    pub chan: i64,
    pub preview: LinkPreview,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoreEvent {
    pub chan: i64,
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default, rename = "totalMessages")]
    pub total_messages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NamesEvent {
    pub id: i64,
    #[serde(default)]
    pub users: Vec<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JoinEvent {
    #[serde(default, rename = "shouldOpen")]
    pub should_open: bool,
    #[serde(default)]
    pub index: i64,
    pub network: String,
    pub chan: Channel,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartEvent {
    pub chan: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuitEvent {
    pub network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelStateEvent {
    pub chan: i64,
    pub state: ChanState,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopicEvent {
    pub chan: i64,
    #[serde(default)]
    pub topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelRenameEvent {
    pub chan: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserAwayEvent {
    pub chan: i64,
    pub nick: String,
    pub away: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsersStatusEvent {
    #[serde(default, rename = "changedChannels")]
    pub changed_channels: Vec<String>,
    #[serde(default, rename = "networkId")]
    pub network_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkEvent {
    pub network: Network,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkOptionsEvent {
    pub network: String,
    #[serde(default, rename = "serverOptions")]
    pub server_options: ServerOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkNameEvent {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NickEvent {
    pub network: String,
    pub nick: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SortingEvent {
    #[serde(default)]
    pub order: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelSortingEvent {
    pub network: String,
    #[serde(default)]
    pub order: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageSpecialEvent {
    pub chan: i64,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingEvent {
    pub name: String,
    #[serde(default)]
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MuteChangedEvent {
    pub target: i64,
    #[serde(default)]
    pub status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistoryClearEvent {
    pub target: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangePasswordEvent {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub error: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum ServerEvent {
    AuthSuccess,
    AuthFailed,
    Init(Box<InitData>),
    Commands(Vec<String>),
    Configuration(Box<Configuration>),
    Network(NetworkEvent),
    NetworkStatus(NetworkStatusEvent),
    NetworkName(NetworkNameEvent),
    NetworkOptions(NetworkOptionsEvent),
    NetworkInfo(NetworkInfo),
    Nick(NickEvent),
    Join(JoinEvent),
    Part(PartEvent),
    Quit(QuitEvent),
    ChannelState(ChannelStateEvent),
    ChannelRename(ChannelRenameEvent),
    Topic(TopicEvent),
    Names(NamesEvent),
    Users(i64),
    UserAway(UserAwayEvent),
    UsersOnline(UsersStatusEvent),
    UsersOffline(UsersStatusEvent),
    Message(Box<MsgEvent>),
    MessagePreview(MessagePreviewEvent),
    MessageSpecial(MessageSpecialEvent),
    More(MoreEvent),
    HistoryClear(HistoryClearEvent),
    MuteChanged(MuteChangedEvent),
    Open(i64),
    SyncSortNetworks(Vec<Value>),
    SyncSortChannels(ChannelSortingEvent),
    SettingNew(SettingEvent),
    SettingAll(serde_json::Map<String, Value>),
    SearchResults(SearchResponse),
    Mentions(Vec<Mention>),
    Typing(TypingEvent),
    Sessions(Vec<Session>),
    UploadAuth(String),
    ChangePassword(ChangePasswordEvent),
    PushSubscribed(bool),
    Changelog(Value),
    ChangelogNewVersion,
    SignedOut,
    Error(Value),
    Other(String, Value),
}

fn as_value<T: for<'de> Deserialize<'de>>(value: &Value) -> Option<T> {
    serde_json::from_value(value.clone()).ok()
}

impl ServerEvent {
    pub fn parse(name: &str, data: Option<&Value>) -> Option<ServerEvent> {
        let value = data.cloned().unwrap_or(Value::Null);

        Some(match name {
            "auth:success" => ServerEvent::AuthSuccess,
            "auth:failed" => ServerEvent::AuthFailed,
            "init" => ServerEvent::Init(as_value(&value)?),
            "commands" => ServerEvent::Commands(as_value(&value)?),
            "configuration" => ServerEvent::Configuration(as_value(&value)?),
            "network" => ServerEvent::Network(as_value(&value)?),
            "network:status" => ServerEvent::NetworkStatus(as_value(&value)?),
            "network:name" => ServerEvent::NetworkName(as_value(&value)?),
            "network:options" => ServerEvent::NetworkOptions(as_value(&value)?),
            "network:info" => ServerEvent::NetworkInfo(as_value(&value)?),
            "nick" => ServerEvent::Nick(as_value(&value)?),
            "join" => ServerEvent::Join(as_value(&value)?),
            "part" => ServerEvent::Part(as_value(&value)?),
            "quit" => ServerEvent::Quit(as_value(&value)?),
            "channel:state" => ServerEvent::ChannelState(as_value(&value)?),
            "channel:rename" => ServerEvent::ChannelRename(as_value(&value)?),
            "topic" => ServerEvent::Topic(as_value(&value)?),
            "names" => ServerEvent::Names(as_value(&value)?),
            "users" => ServerEvent::Users(as_value(&value)?),
            "user:away" => ServerEvent::UserAway(as_value(&value)?),
            "users:online" => ServerEvent::UsersOnline(as_value(&value)?),
            "users:offline" => ServerEvent::UsersOffline(as_value(&value)?),
            "msg" => ServerEvent::Message(Box::new(as_value(&value)?)),
            "msg:preview" => ServerEvent::MessagePreview(as_value(&value)?),
            "msg:special" => ServerEvent::MessageSpecial(as_value(&value)?),
            "more" => ServerEvent::More(as_value(&value)?),
            "history:clear" => ServerEvent::HistoryClear(as_value(&value)?),
            "mute:changed" => ServerEvent::MuteChanged(as_value(&value)?),
            "open" => ServerEvent::Open(as_value(&value).unwrap_or(0)),
            "sync_sort:networks" => {
                ServerEvent::SyncSortNetworks(as_value::<SortingEvent>(&value)?.order)
            }
            "sync_sort:channels" => ServerEvent::SyncSortChannels(as_value(&value)?),
            "setting:new" => ServerEvent::SettingNew(as_value(&value)?),
            "setting:all" => ServerEvent::SettingAll(as_value(&value)?),
            "search:results" => ServerEvent::SearchResults(as_value(&value)?),
            "mentions:list" => ServerEvent::Mentions(as_value(&value)?),
            "typing" => ServerEvent::Typing(as_value(&value)?),
            "sessions:list" => ServerEvent::Sessions(as_value(&value)?),
            "upload:auth" => ServerEvent::UploadAuth(as_value(&value)?),
            "change-password" => ServerEvent::ChangePassword(as_value(&value)?),
            "push:issubscribed" => ServerEvent::PushSubscribed(as_value(&value)?),
            "changelog" => ServerEvent::Changelog(value),
            "changelog:newversion" => ServerEvent::ChangelogNewVersion,
            "sign-out" => ServerEvent::SignedOut,
            "error" => ServerEvent::Error(value),
            other => ServerEvent::Other(other.to_string(), value),
        })
    }
}

pub fn socket_url(base: &str) -> Result<url::Url, url::ParseError> {
    let mut url = url::Url::parse(base)?;
    let scheme = match url.scheme() {
        "https" | "wss" => "wss",
        _ => "ws",
    };

    let mut path = url.path().trim_end_matches('/').to_string();
    path.push_str("/socket.io/");
    url.set_path(&path);
    url.set_query(None);
    url.set_fragment(None);
    url.set_scheme(scheme).ok();

    url.query_pairs_mut()
        .append_pair("EIO", "4")
        .append_pair("transport", "websocket");
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_url_root() {
        let u = socket_url("http://localhost:9000").unwrap();
        assert_eq!(u.scheme(), "ws");
        assert_eq!(u.path(), "/socket.io/");
        assert!(u.query().unwrap().contains("EIO=4"));
    }

    #[test]
    fn socket_url_subpath_and_tls() {
        let u = socket_url("https://example.com/lounge/").unwrap();
        assert_eq!(u.scheme(), "wss");
        assert_eq!(u.path(), "/lounge/socket.io/");
    }

    #[test]
    fn parses_init_shape() {
        let json = serde_json::json!({
            "active": 2,
            "token": "abc",
            "networks": [{
                "uuid": "u1",
                "name": "Libera",
                "nick": "me",
                "status": {"connected": true, "secure": true},
                "serverOptions": {
                    "CHANTYPES": ["#"],
                    "PREFIX": {"prefix": [{"symbol": "@", "mode": "o"}], "modeToSymbol": {"o": "@"}, "symbols": ["@"]},
                    "NETWORK": "Libera.Chat",
                    "supportsReply": true,
                    "MONITOR": 100
                },
                "channels": [{
                    "id": 2,
                    "name": "#rust",
                    "key": "",
                    "topic": "the topic",
                    "firstUnread": 0,
                    "unread": 3,
                    "highlight": 1,
                    "muted": false,
                    "type": "channel",
                    "state": 1,
                    "messages": [{
                        "id": 42,
                        "text": "hello",
                        "type": "message",
                        "self": false,
                        "time": "2026-11-05T12:00:00.000Z",
                        "from": {"nick": "alice", "mode": "o"},
                        "users": []
                    }],
                    "totalMessages": 120,
                    "num_users": 9
                }]
            }]
        });
        let init: InitData = serde_json::from_value(json).unwrap();
        assert_eq!(init.active, 2);
        assert_eq!(init.networks[0].channels[0].kind, ChanType::Channel);
        assert_eq!(init.networks[0].channels[0].state, ChanState::Joined);
        assert_eq!(
            init.networks[0].channels[0].messages[0].kind,
            MessageType::Message
        );
        assert!(
            init.networks[0]
                .server_options
                .as_ref()
                .unwrap()
                .supports_reply
        );
    }

    #[test]
    fn parses_link_preview_with_negative_size() {
        let message: Message = serde_json::from_value(serde_json::json!({
            "id": 1,
            "text": "https://example.com",
            "type": "message",
            "previews": [{
                "type": "link",
                "head": "Example",
                "body": "",
                "thumb": "",
                "size": -1,
                "link": "https://example.com",
                "shown": null
            }]
        }))
        .unwrap();
        assert_eq!(message.previews.len(), 1);
        assert_eq!(message.previews[0].size, -1);
        assert_eq!(message.previews[0].link, "https://example.com");
    }

    #[test]
    fn parses_events() {
        let ev = ServerEvent::parse("open", Some(&serde_json::json!(7))).unwrap();
        assert!(matches!(ev, ServerEvent::Open(7)));

        let ev = ServerEvent::parse(
            "channel:state",
            Some(&serde_json::json!({"chan": 2, "state": 0})),
        )
        .unwrap();
        assert!(matches!(ev, ServerEvent::ChannelState(s) if s.state == ChanState::Parted));
    }
}
