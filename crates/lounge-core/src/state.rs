use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::proto::*;

const MAX_MESSAGES_PER_CHANNEL: usize = 2_000;

#[derive(Debug, Clone, Default)]
pub struct NetworkState {
    pub uuid: String,
    pub name: String,
    pub nick: String,
    pub status: NetworkStatus,
    pub server_options: Option<ServerOptions>,
    pub channels: Vec<ChannelState>,
}

#[derive(Debug, Clone, Default)]
pub struct ChannelState {
    pub chan: Channel,
    pub users: Vec<User>,
    pub users_outdated: bool,
    pub history_loading: bool,
    pub typing: Vec<TypingUser>,
}

impl ChannelState {
    pub fn is_channel(&self) -> bool {
        self.chan.kind == ChanType::Channel
    }

    pub fn is_query(&self) -> bool {
        self.chan.kind == ChanType::Query
    }

    pub fn is_lobby(&self) -> bool {
        self.chan.kind == ChanType::Lobby
    }

    pub fn more_history_available(&self) -> bool {
        self.chan.total_messages > self.chan.messages.len()
    }
}

#[derive(Debug, Clone)]
pub struct TypingUser {
    pub nick: String,
    pub status: TypingStatus,
    pub updated: Instant,
}

#[derive(Debug, Clone, Default)]
pub struct ClientState {
    pub networks: Vec<NetworkState>,
    pub active: Option<i64>,
    pub commands: Vec<String>,
    pub configuration: Option<Configuration>,
    pub settings: HashMap<String, Value>,
    pub mentions: Vec<Mention>,
    pub search_results: Option<SearchResponse>,
    pub network_info: Option<NetworkInfo>,
    pub sessions: Vec<Session>,
    pub upload_token: Option<String>,
    pub signed_out: bool,
    pub auth_failed: bool,
    pub last_error: Option<String>,
    pub max_msg_id: i64,
    pub token: Option<String>,

    pub revision: u64,
}

impl ClientState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, event: ServerEvent) {
        self.revision += 1;
        match event {
            ServerEvent::AuthSuccess => {}
            ServerEvent::AuthFailed => {
                self.auth_failed = true;
                self.token = None;
            }
            ServerEvent::Init(data) => self.apply_init(*data),
            ServerEvent::Commands(commands) => self.commands = commands,
            ServerEvent::Configuration(config) => self.configuration = Some(*config),
            ServerEvent::Network(event) => self.apply_network(event.network),
            ServerEvent::NetworkStatus(event) => {
                if let Some(network) = self.networks.iter_mut().find(|n| n.uuid == event.network) {
                    network.status.connected = event.connected;
                    network.status.secure = event.secure;
                    if !event.connected {
                        for channel in &mut network.channels {
                            channel.users.clear();
                            channel.chan.state = ChanState::Parted;
                            channel.users_outdated = true;
                        }
                    }
                }
            }
            ServerEvent::NetworkName(event) => {
                if let Some(network) = self.networks.iter_mut().find(|n| n.uuid == event.uuid) {
                    network.name = event.name.clone();
                    if let Some(first) = network.channels.first_mut() {
                        first.chan.name = event.name;
                    }
                }
            }
            ServerEvent::NetworkOptions(event) => {
                if let Some(network) = self.networks.iter_mut().find(|n| n.uuid == event.network) {
                    network.server_options = Some(event.server_options);
                }
            }
            ServerEvent::NetworkInfo(info) => self.network_info = Some(info),
            ServerEvent::Nick(event) => {
                if let Some(network) = self.networks.iter_mut().find(|n| n.uuid == event.network) {
                    network.nick = event.nick;
                }
            }
            ServerEvent::Join(event) => self.apply_join(event),
            ServerEvent::Part(event) => self.apply_part(event.chan),
            ServerEvent::Quit(event) => self.apply_quit(&event.network),
            ServerEvent::ChannelState(event) => {
                if let Some(channel) = self.find_channel_mut(event.chan) {
                    channel.chan.state = event.state;
                }
            }
            ServerEvent::ChannelRename(event) => {
                if let Some(channel) = self.find_channel_mut(event.chan) {
                    channel.chan.name = event.name;
                }
            }
            ServerEvent::Topic(event) => {
                if let Some(channel) = self.find_channel_mut(event.chan) {
                    channel.chan.topic = event.topic;
                }
            }
            ServerEvent::Names(event) => self.apply_names(event),
            ServerEvent::Users(chan) => {
                if let Some(channel) = self.find_channel_mut(chan) {
                    channel.users_outdated = true;
                }
            }
            ServerEvent::UserAway(event) => {
                if let Some(channel) = self.find_channel_mut(event.chan) {
                    channel.chan.user_away = event.away.clone();
                    let nick = event.nick.to_lowercase();
                    if let Some(user) = channel
                        .users
                        .iter_mut()
                        .find(|u| u.nick.to_lowercase() == nick)
                    {
                        user.away = event.away.unwrap_or_default();
                    }
                }
            }
            ServerEvent::UsersOnline(event) => self.apply_monitor_status(&event, true),
            ServerEvent::UsersOffline(event) => self.apply_monitor_status(&event, false),
            ServerEvent::Message(event) => self.apply_message(*event),
            ServerEvent::MessagePreview(event) => {
                if let Some(channel) = self.find_channel_mut(event.chan)
                    && let Some(message) = channel
                        .chan
                        .messages
                        .iter_mut()
                        .find(|message| message.id == event.id)
                {
                    if let Some(existing) = message
                        .previews
                        .iter_mut()
                        .find(|preview| preview.link == event.preview.link)
                    {
                        *existing = event.preview;
                    } else {
                        message.previews.push(event.preview);
                    }
                }
            }
            ServerEvent::MessageSpecial(event) => {
                if let Some(channel) = self.find_channel_mut(event.chan) {
                    channel.chan.data = event.data;
                }
                self.active = Some(event.chan);
            }
            ServerEvent::More(event) => {
                if let Some(channel) = self.find_channel_mut(event.chan) {
                    channel.history_loading = false;
                    channel.chan.total_messages = event.total_messages;
                    merge_messages(&mut channel.chan.messages, event.messages);
                    trim_messages(&mut channel.chan.messages);
                }
            }
            ServerEvent::HistoryClear(event) => {
                if let Some(channel) = self.find_channel_mut(event.target) {
                    channel.chan.messages.clear();
                    channel.chan.total_messages = 0;
                    channel.chan.first_unread = 0;
                    channel.chan.unread = 0;
                    channel.chan.highlight = 0;
                }
            }
            ServerEvent::MuteChanged(event) => {
                if let Some(channel) = self.find_channel_mut(event.target) {
                    channel.chan.muted = event.status;
                }
            }
            ServerEvent::Open(id) => {
                if Some(id) != self.active
                    && let Some(channel) = self.find_channel_mut(id)
                {
                    channel.chan.unread = 0;
                    channel.chan.highlight = 0;
                    if let Some(last) = channel.chan.messages.last() {
                        channel.chan.first_unread = last.id;
                    }
                }
            }
            ServerEvent::SyncSortNetworks(order) => {
                let order: Vec<String> = order
                    .into_iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect();
                self.networks.sort_by_key(|network| {
                    order
                        .iter()
                        .position(|uuid| *uuid == network.uuid)
                        .unwrap_or(usize::MAX)
                });
            }
            ServerEvent::SyncSortChannels(event) => {
                if let Some(network) = self.networks.iter_mut().find(|n| n.uuid == event.network) {
                    network.channels.sort_by_key(|channel| {
                        event
                            .order
                            .iter()
                            .position(|id| *id == channel.chan.id)
                            .unwrap_or(usize::MAX)
                    });
                }
            }
            ServerEvent::SettingNew(event) => {
                self.settings.insert(event.name, event.value);
            }
            ServerEvent::SettingAll(settings) => {
                self.settings = settings.into_iter().collect();
            }
            ServerEvent::SearchResults(results) => self.search_results = Some(results),
            ServerEvent::Mentions(mentions) => self.mentions = mentions,
            ServerEvent::Typing(event) => self.apply_typing(event),
            ServerEvent::Sessions(sessions) => self.sessions = sessions,
            ServerEvent::UploadAuth(token) => self.upload_token = Some(token),
            ServerEvent::ChangePassword(_) => {}
            ServerEvent::PushSubscribed(_) => {}
            ServerEvent::Changelog(_) | ServerEvent::ChangelogNewVersion => {}
            ServerEvent::SignedOut => {
                self.signed_out = true;
                self.token = None;
            }
            ServerEvent::Error(error) => {
                self.last_error = Some(match &error {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                });
            }
            ServerEvent::Other(_, _) => {}
        }
        self.prune_typing(Duration::from_secs(10));
    }

    fn apply_init(&mut self, init: InitData) {
        if let Some(token) = init.token.clone() {
            self.token = Some(token);
        }
        if !init.networks.is_empty() {
            self.merge_networks(init.networks);
        }

        if init.active > 0 && self.find_channel(init.active).is_some() {
            self.active = Some(init.active);
        } else if self.active.is_none() || self.find_channel(self.active.unwrap()).is_none() {
            self.active = self
                .networks
                .first()
                .and_then(|network| network.channels.first())
                .map(|channel| channel.chan.id);
        }

        for network in &self.networks {
            for channel in &network.channels {
                self.max_msg_id = self.max_msg_id.max(
                    channel
                        .chan
                        .messages
                        .iter()
                        .map(|m| m.id)
                        .max()
                        .unwrap_or(0),
                );
            }
        }
    }

    fn merge_networks(&mut self, incoming: Vec<Network>) {
        let old = std::mem::take(&mut self.networks);
        let mut merged = Vec::with_capacity(incoming.len());

        for network in incoming {
            let existing = old.iter().find(|n| n.uuid == network.uuid);
            let mut state = NetworkState {
                uuid: network.uuid.clone(),
                name: network.name.clone(),
                nick: network.nick.clone(),
                status: network.status,
                server_options: network.server_options.clone(),
                channels: Vec::new(),
            };

            for channel in network.channels {
                let existing_channel =
                    existing.and_then(|n| n.channels.iter().find(|c| c.chan.id == channel.id));
                let mut state_channel = match existing_channel {
                    Some(old_channel) => {
                        let users = old_channel.users.clone();
                        let typing = old_channel.typing.clone();
                        ChannelState {
                            chan: channel,
                            users,
                            users_outdated: true,
                            history_loading: false,
                            typing,
                        }
                    }
                    None => ChannelState {
                        chan: channel,
                        users: Vec::new(),
                        users_outdated: true,
                        history_loading: false,
                        typing: Vec::new(),
                    },
                };
                if let Some(old_channel) = existing_channel {
                    merge_messages(
                        &mut state_channel.chan.messages,
                        old_channel.chan.messages.clone(),
                    );
                }
                trim_messages(&mut state_channel.chan.messages);
                state.channels.push(state_channel);
            }

            merged.push(state);
        }

        self.networks = merged;
    }

    fn apply_network(&mut self, network: Network) {
        let mut state = NetworkState {
            uuid: network.uuid.clone(),
            name: network.name.clone(),
            nick: network.nick.clone(),
            status: network.status,
            server_options: network.server_options,
            channels: network
                .channels
                .into_iter()
                .map(new_channel_state)
                .collect(),
        };
        for channel in &mut state.channels {
            self.max_msg_id = self.max_msg_id.max(
                channel
                    .chan
                    .messages
                    .iter()
                    .map(|m| m.id)
                    .max()
                    .unwrap_or(0),
            );
        }
        let active = state.channels.last().map(|c| c.chan.id);
        self.networks.push(state);
        if let Some(active) = active {
            self.active = Some(active);
        }
    }

    fn apply_join(&mut self, event: JoinEvent) {
        let Some(network) = self.networks.iter_mut().find(|n| n.uuid == event.network) else {
            return;
        };
        let channel_id = event.chan.id;
        let mut channel = new_channel_state(event.chan);
        channel.users_outdated = true;
        if let Some(existing) = network
            .channels
            .iter_mut()
            .find(|c| c.chan.id == channel_id)
        {
            merge_messages(&mut channel.chan.messages, existing.chan.messages.clone());
            channel.users = std::mem::take(&mut existing.users);
            *existing = channel;
        } else {
            let index = event.index.clamp(0, network.channels.len() as i64) as usize;
            network.channels.insert(index, channel);
        }

        let is_query = network
            .channels
            .iter()
            .find(|c| c.chan.id == channel_id)
            .map(|c| c.is_query())
            .unwrap_or(false);
        if !is_query || event.should_open {
            self.active = Some(channel_id);
        }
    }

    fn apply_part(&mut self, chan_id: i64) {
        let network_uuid = self.network_uuid_for_chan(chan_id);
        let was_active = self.active == Some(chan_id);

        for network in &mut self.networks {
            network.channels.retain(|c| c.chan.id != chan_id);
        }

        if was_active {
            let fallback = network_uuid
                .as_deref()
                .and_then(|uuid| self.networks.iter().find(|n| n.uuid == uuid))
                .and_then(|network| network.channels.first())
                .map(|channel| channel.chan.id)
                .or_else(|| {
                    self.networks
                        .iter()
                        .flat_map(|n| n.channels.iter())
                        .next()
                        .map(|channel| channel.chan.id)
                });
            self.active = fallback;
        }
    }

    fn apply_quit(&mut self, network_uuid: &str) {
        let was_active_network = self
            .active
            .and_then(|id| self.network_uuid_for_chan(id))
            .as_deref()
            == Some(network_uuid);
        self.networks.retain(|n| n.uuid != network_uuid);
        if was_active_network {
            self.active = self
                .networks
                .first()
                .and_then(|n| n.channels.first())
                .map(|c| c.chan.id);
        }
    }

    fn apply_names(&mut self, event: NamesEvent) {
        let options = self
            .network_uuid_for_chan(event.id)
            .as_deref()
            .and_then(|uuid| self.networks.iter().find(|n| n.uuid == uuid))
            .and_then(|network| network.server_options.clone());
        if let Some(channel) = self.find_channel_mut(event.id) {
            let mut users = event.users;
            sort_users(&mut users, options.as_ref());
            channel.users = users;
            channel.users_outdated = false;
        }
    }

    fn apply_monitor_status(&mut self, event: &UsersStatusEvent, online: bool) {
        if let Some(network) = self
            .networks
            .iter_mut()
            .find(|n| n.uuid == event.network_id)
        {
            for channel in &mut network.channels {
                if event.changed_channels.contains(&channel.chan.name) {
                    channel.chan.is_online = Some(online);
                }
            }
        }
    }

    fn apply_message(&mut self, mut event: MsgEvent) {
        let Some(network_uuid) = self.network_uuid_for_chan(event.chan) else {
            return;
        };
        if self.find_channel(event.chan).is_none() {
            return;
        }

        let active_network = self.active.and_then(|id| self.network_uuid_for_chan(id));
        let mut display_chan = event.chan;
        let mut is_active = self.active == Some(event.chan);

        if event.msg.show_in_active && active_network.as_deref() == Some(network_uuid.as_str()) {
            if let Some(active) = self.active {
                if active == event.chan {
                    event.msg.show_in_active = false;
                } else {
                    display_chan = active;
                    is_active = true;
                }
            }
        } else {
            event.msg.show_in_active = false;
        }

        if !is_active {
            let highlight = event.highlight;
            let unread = event.unread;
            if let Some(channel) = self.find_channel_mut(event.chan) {
                if let Some(highlight) = highlight {
                    channel.chan.highlight = highlight;
                }
                if let Some(unread) = unread {
                    channel.chan.unread = unread;
                }
            }
        }

        let id = event.msg.id;
        let is_self = event.msg.is_self;
        if let Some(channel) = self.find_channel_mut(event.chan) {
            channel.chan.total_messages = channel.chan.total_messages.saturating_add(1);
            if is_self {
                channel.chan.first_unread = id;
            }
        }

        if let Some(channel) = self.find_channel_mut(display_chan) {
            let mut msg = event.msg;
            if msg.id > 0
                && channel
                    .chan
                    .messages
                    .iter()
                    .any(|existing| existing.id == msg.id)
            {
                return;
            }
            if msg.id <= 0 {
                msg.id = channel.chan.messages.last().map(|m| m.id + 1).unwrap_or(1);
            }
            channel.chan.messages.push(msg);
            trim_messages(&mut channel.chan.messages);
        }

        self.max_msg_id = self.max_msg_id.max(id);
    }

    fn apply_typing(&mut self, event: TypingEvent) {
        let Some(channel) = self.find_channel_mut(event.chan) else {
            return;
        };
        channel.typing.retain(|t| t.nick != event.nick);
        if event.status != TypingStatus::Done {
            channel.typing.push(TypingUser {
                nick: event.nick,
                status: event.status,
                updated: Instant::now(),
            });
        }
    }

    pub fn prune_typing(&mut self, max_age: Duration) {
        let now = Instant::now();
        for network in &mut self.networks {
            for channel in &mut network.channels {
                channel
                    .typing
                    .retain(|t| now.duration_since(t.updated) < max_age);
            }
        }
    }

    pub fn find_channel(&self, id: i64) -> Option<(&NetworkState, &ChannelState)> {
        for network in &self.networks {
            if let Some(channel) = network.channels.iter().find(|c| c.chan.id == id) {
                return Some((network, channel));
            }
        }
        None
    }

    pub fn find_channel_mut(&mut self, id: i64) -> Option<&mut ChannelState> {
        for network in &mut self.networks {
            if let Some(index) = network.channels.iter().position(|c| c.chan.id == id) {
                return Some(&mut network.channels[index]);
            }
        }
        None
    }

    pub fn network_uuid_for_chan(&self, id: i64) -> Option<String> {
        self.networks
            .iter()
            .find(|network| network.channels.iter().any(|c| c.chan.id == id))
            .map(|network| network.uuid.clone())
    }

    pub fn network_for_chan(&self, id: i64) -> Option<&NetworkState> {
        self.networks
            .iter()
            .find(|network| network.channels.iter().any(|c| c.chan.id == id))
    }

    pub fn active_channel(&self) -> Option<(&NetworkState, &ChannelState)> {
        self.active.and_then(|id| self.find_channel(id))
    }

    pub fn active_network(&self) -> Option<&NetworkState> {
        self.active.and_then(|id| self.network_for_chan(id))
    }

    pub fn set_active(&mut self, id: i64) {
        if self.find_channel(id).is_none() {
            return;
        }
        self.active = Some(id);
        if let Some(channel) = self.find_channel_mut(id) {
            channel.chan.unread = 0;
            channel.chan.highlight = 0;
            if let Some(last) = channel.chan.messages.last() {
                channel.chan.first_unread = last.id;
            }
        }
    }

    pub fn total_unread(&self) -> u32 {
        self.networks
            .iter()
            .flat_map(|network| network.channels.iter())
            .map(|channel| channel.chan.unread + channel.chan.highlight)
            .sum()
    }

    pub fn total_highlights(&self) -> u32 {
        self.networks
            .iter()
            .flat_map(|network| network.channels.iter())
            .map(|channel| channel.chan.highlight)
            .sum()
    }

    pub fn nicks_for_active(&self) -> Vec<String> {
        self.active_channel()
            .map(|(_, channel)| channel.users.iter().map(|u| u.nick.clone()).collect())
            .unwrap_or_default()
    }

    pub fn mark_active_history_requested(&mut self) -> Option<(i64, i64)> {
        let id = self.active?;
        let channel = self.find_channel_mut(id)?;
        if channel.history_loading || !channel.more_history_available() {
            return None;
        }
        channel.history_loading = true;
        let last_id = channel
            .chan
            .messages
            .iter()
            .find(|m| !m.show_in_active)
            .map(|m| m.id)
            .or_else(|| channel.chan.messages.first().map(|m| m.id))
            .unwrap_or(-1);
        Some((id, last_id))
    }

    pub fn mark_history_requested(&mut self, chan_id: i64) -> Option<i64> {
        let channel = self.find_channel_mut(chan_id)?;
        if channel.history_loading || !channel.more_history_available() {
            return None;
        }
        channel.history_loading = true;
        Some(
            channel
                .chan
                .messages
                .iter()
                .find(|m| !m.show_in_active)
                .map(|m| m.id)
                .or_else(|| channel.chan.messages.first().map(|m| m.id))
                .unwrap_or(-1),
        )
    }
}

fn new_channel_state(chan: Channel) -> ChannelState {
    ChannelState {
        chan,
        users: Vec::new(),
        users_outdated: true,
        history_loading: false,
        typing: Vec::new(),
    }
}

fn merge_messages(existing: &mut Vec<Message>, incoming: Vec<Message>) {
    if existing.is_empty() {
        *existing = incoming;
        return;
    }
    let mut changed = false;
    for message in incoming {
        if message.id > 0 && existing.iter().any(|m| m.id == message.id) {
            continue;
        }
        existing.push(message);
        changed = true;
    }
    if changed {
        existing.sort_by_key(|m| m.id);
    }
}

fn trim_messages(messages: &mut Vec<Message>) {
    if messages.len() > MAX_MESSAGES_PER_CHANNEL {
        let remove = messages.len() - MAX_MESSAGES_PER_CHANNEL;
        messages.drain(..remove);
    }
}

fn sort_users(users: &mut [User], options: Option<&ServerOptions>) {
    let prefixes = options
        .map(|options| {
            options
                .prefix
                .prefix
                .iter()
                .map(|prefix| prefix.mode.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    users.sort_by(|a, b| {
        let rank = |user: &User| {
            prefixes
                .iter()
                .position(|mode| user.modes.contains(mode))
                .unwrap_or(usize::MAX)
        };
        rank(a)
            .cmp(&rank(b))
            .then_with(|| a.nick.to_lowercase().cmp(&b.nick.to_lowercase()))
    });
}

#[cfg(test)]
mod tests;
