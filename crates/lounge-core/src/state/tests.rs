use super::*;
use serde_json::json;

fn init_state() -> ClientState {
    let mut state = ClientState::new();
    let init: InitData = serde_json::from_value(json!({
        "active": 2,
        "token": "tok",
        "networks": [{
            "uuid": "u1",
            "name": "Libera",
            "nick": "me",
            "status": {"connected": true, "secure": true},
            "serverOptions": {
                "CHANTYPES": ["#"],
                "PREFIX": {"prefix": [{"symbol": "@", "mode": "o"}, {"symbol": "+", "mode": "v"}], "modeToSymbol": {"o": "@", "v": "+"}, "symbols": ["@", "+"]},
                "NETWORK": "Libera.Chat",
                "supportsReply": true
            },
            "channels": [
                {"id": 1, "name": "Libera", "key": "", "topic": "", "firstUnread": 0, "unread": 0, "highlight": 0, "muted": false, "type": "lobby", "state": 1, "messages": [], "totalMessages": 0},
                {"id": 2, "name": "#rust", "key": "", "topic": "Rust", "firstUnread": 0, "unread": 4, "highlight": 1, "muted": false, "type": "channel", "state": 1, "messages": [
                    {"id": 10, "text": "hello", "type": "message", "time": "2026-11-05T12:00:00Z", "from": {"nick": "alice", "mode": "o"}},
                    {"id": 11, "text": "hi", "type": "message", "time": "2026-11-05T12:01:00Z", "from": {"nick": "bob", "mode": "v"}}
                ], "totalMessages": 42}
            ]
        }]
    })).unwrap();
    state.apply(ServerEvent::Init(Box::new(init)));
    state
}

#[test]
fn init_sets_active_and_token() {
    let state = init_state();
    assert_eq!(state.active, Some(2));
    assert_eq!(state.token.as_deref(), Some("tok"));
    assert_eq!(state.find_channel(2).unwrap().1.chan.messages.len(), 2);
    assert_eq!(state.max_msg_id, 11);
    assert!(state.find_channel(2).unwrap().1.more_history_available());
}

#[test]
fn message_updates_unread_and_active() {
    let mut state = init_state();
    state.set_active(1);
    state.apply(ServerEvent::Message(Box::new(MsgEvent {
        chan: 2,
        msg: Message {
            id: 12,
            text: "new".into(),
            kind: MessageType::Message,
            from: Some(UserInMessage {
                nick: "carol".into(),
                ..Default::default()
            }),
            ..Default::default()
        },
        highlight: Some(2),
        unread: Some(5),
    })));
    assert_eq!(state.find_channel(2).unwrap().1.chan.unread, 5);
    assert_eq!(state.find_channel(2).unwrap().1.chan.highlight, 2);
    assert_eq!(state.find_channel(2).unwrap().1.chan.total_messages, 43);
    assert_eq!(state.max_msg_id, 12);

    state.set_active(2);
    assert_eq!(state.find_channel(2).unwrap().1.chan.unread, 0);
    state.apply(ServerEvent::Message(Box::new(MsgEvent {
        chan: 2,
        msg: Message {
            id: 13,
            text: "active".into(),
            from: Some(UserInMessage {
                nick: "carol".into(),
                ..Default::default()
            }),
            ..Default::default()
        },
        highlight: Some(3),
        unread: Some(9),
    })));

    assert_eq!(state.find_channel(2).unwrap().1.chan.unread, 0);
    assert_eq!(state.find_channel(2).unwrap().1.chan.highlight, 0);
}

#[test]
fn show_in_active_redirects_display() {
    let mut state = init_state();
    state.set_active(1);
    state.apply(ServerEvent::Message(Box::new(MsgEvent {
        chan: 2,
        msg: Message {
            id: 99,
            text: "notice".into(),
            kind: MessageType::Notice,
            show_in_active: true,
            from: Some(UserInMessage {
                nick: "x".into(),
                ..Default::default()
            }),
            ..Default::default()
        },
        unread: Some(3),
        highlight: Some(1),
    })));
    assert_eq!(state.find_channel(1).unwrap().1.chan.messages.len(), 1);
    assert_eq!(state.find_channel(1).unwrap().1.chan.messages[0].id, 99);

    assert_eq!(state.find_channel(2).unwrap().1.chan.unread, 4);
    assert!(state.find_channel(1).unwrap().1.chan.messages[0].show_in_active);
}

#[test]
fn more_prepends_and_dedupes() {
    let mut state = init_state();
    state.apply(ServerEvent::More(MoreEvent {
        chan: 2,
        messages: vec![
            Message {
                id: 8,
                text: "old".into(),
                ..Default::default()
            },
            Message {
                id: 10,
                text: "hello".into(),
                ..Default::default()
            },
            Message {
                id: 9,
                text: "older".into(),
                ..Default::default()
            },
        ],
        total_messages: 42,
    }));
    let messages = &state.find_channel(2).unwrap().1.chan.messages;
    assert_eq!(
        messages.iter().map(|m| m.id).collect::<Vec<_>>(),
        vec![8, 9, 10, 11]
    );
    assert!(!state.find_channel(2).unwrap().1.history_loading);
}

#[test]
fn part_and_quit_pick_fallback() {
    let mut state = init_state();
    state.apply(ServerEvent::Part(PartEvent { chan: 2 }));
    assert_eq!(state.active, Some(1));
    assert!(state.find_channel(2).is_none());

    state.apply(ServerEvent::Quit(QuitEvent {
        network: "u1".into(),
    }));
    assert_eq!(state.active, None);
    assert!(state.networks.is_empty());
}

#[test]
fn total_unread_does_not_double_count_highlights() {
    let state = init_state();
    assert_eq!(state.total_unread(), 4);
    assert_eq!(state.total_highlights(), 1);
}

#[test]
fn empty_init_clears_stale_state() {
    let mut state = init_state();
    let init: InitData = serde_json::from_value(json!({
        "active": 0,
        "networks": []
    }))
    .unwrap();
    state.apply(ServerEvent::Init(Box::new(init)));
    assert!(state.networks.is_empty());
    assert_eq!(state.active, None);
}

#[test]
fn names_sorted_by_mode_then_nick() {
    let mut state = init_state();
    state.apply(ServerEvent::Names(NamesEvent {
        id: 2,
        users: vec![
            User {
                nick: "zed".into(),
                modes: vec!["+".into()],
                ..Default::default()
            },
            User {
                nick: "bob".into(),
                modes: vec!["@".into()],
                away: String::new(),
                ..Default::default()
            },
            User {
                nick: "alice".into(),
                ..Default::default()
            },
        ],
    }));
    let users = &state.find_channel(2).unwrap().1.users;
    assert_eq!(
        users.iter().map(|u| u.nick.as_str()).collect::<Vec<_>>(),
        vec!["bob", "zed", "alice"]
    );
    assert!(!state.find_channel(2).unwrap().1.users_outdated);
}

#[test]
fn sorting_events_reorder() {
    let mut state = init_state();
    state.apply(ServerEvent::SyncSortChannels(ChannelSortingEvent {
        network: "u1".into(),
        order: vec![2, 1],
    }));
    let ids: Vec<i64> = state.networks[0]
        .channels
        .iter()
        .map(|c| c.chan.id)
        .collect();
    assert_eq!(ids, vec![2, 1]);
    state.apply(ServerEvent::SyncSortNetworks(vec![json!("u1")]));
    assert_eq!(state.networks[0].uuid, "u1");
}

#[test]
fn join_inserts_and_opens_channel() {
    let mut state = init_state();
    state.apply(ServerEvent::Join(JoinEvent {
        should_open: true,
        index: 2,
        network: "u1".into(),
        chan: Channel {
            id: 3,
            name: "#new".into(),
            kind: ChanType::Channel,
            state: ChanState::Joined,
            ..Default::default()
        },
    }));
    assert_eq!(state.active, Some(3));
    let network = &state.networks[0];
    assert_eq!(network.channels[2].chan.id, 3);
}

#[test]
fn init_reconnect_merges_messages() {
    let mut state = init_state();
    let init: InitData = serde_json::from_value(json!({
        "active": 2,
        "networks": [{
            "uuid": "u1",
            "name": "Libera",
            "nick": "me2",
            "status": {"connected": true, "secure": true},
            "channels": [{
                "id": 2, "name": "#rust", "key": "", "topic": "Rust", "firstUnread": 0,
                "unread": 8, "highlight": 2, "muted": false, "type": "channel", "state": 1,
                "messages": [
                    {"id": 11, "text": "hi", "type": "message", "from": {"nick": "bob", "mode": "v"}},
                    {"id": 12, "text": "reconnected", "type": "message", "from": {"nick": "carol", "mode": ""}}
                ],
                "totalMessages": 43
            }]
        }]
    })).unwrap();
    state.apply(ServerEvent::Init(Box::new(init)));
    assert_eq!(state.networks[0].nick, "me2");
    let channel = state.find_channel(2).unwrap().1;
    assert_eq!(channel.chan.messages.len(), 3);
    assert_eq!(channel.chan.messages.last().unwrap().id, 12);
    assert_eq!(channel.chan.unread, 8);
    assert!(channel.users_outdated);
}

#[test]
fn message_preview_updates_message() {
    let mut state = init_state();
    state.apply(ServerEvent::MessagePreview(MessagePreviewEvent {
        id: 10,
        chan: 2,
        preview: LinkPreview {
            kind: "link".into(),
            head: "Example".into(),
            link: "https://example.com".into(),
            shown: None,
            ..Default::default()
        },
    }));
    let message = state
        .find_channel(2)
        .unwrap()
        .1
        .chan
        .messages
        .iter()
        .find(|message| message.id == 10)
        .unwrap();
    assert_eq!(message.previews.len(), 1);
    assert_eq!(message.previews[0].head, "Example");

    state.apply(ServerEvent::MessagePreview(MessagePreviewEvent {
        id: 10,
        chan: 2,
        preview: LinkPreview {
            kind: "link".into(),
            head: "Updated".into(),
            link: "https://example.com".into(),
            ..Default::default()
        },
    }));
    let message = state
        .find_channel(2)
        .unwrap()
        .1
        .chan
        .messages
        .iter()
        .find(|message| message.id == 10)
        .unwrap();
    assert_eq!(message.previews.len(), 1);
    assert_eq!(message.previews[0].head, "Updated");
}

#[test]
fn typing_expires() {
    let mut state = init_state();
    state.apply(ServerEvent::Typing(TypingEvent {
        network: "u1".into(),
        chan: 2,
        nick: "alice".into(),
        status: TypingStatus::Active,
    }));
    assert_eq!(state.find_channel(2).unwrap().1.typing.len(), 1);
    state.apply(ServerEvent::Typing(TypingEvent {
        network: "u1".into(),
        chan: 2,
        nick: "alice".into(),
        status: TypingStatus::Done,
    }));
    assert!(state.find_channel(2).unwrap().1.typing.is_empty());
}

#[test]
fn typing_expires_by_age() {
    let mut state = init_state();
    state.apply(ServerEvent::Typing(TypingEvent {
        network: "u1".into(),
        chan: 2,
        nick: "alice".into(),
        status: TypingStatus::Active,
    }));
    state
        .find_channel_mut(2)
        .unwrap()
        .typing
        .first_mut()
        .unwrap()
        .updated = Instant::now() - Duration::from_secs(20);
    state.prune_typing(Duration::from_secs(10));
    assert!(state.find_channel(2).unwrap().1.typing.is_empty());
}

#[test]
fn more_keeps_loaded_history_at_capacity() {
    let mut state = init_state();
    {
        let channel = state.find_channel_mut(2).unwrap();
        channel.chan.messages = (1..=2000)
            .map(|id| Message {
                id,
                text: format!("message {id}"),
                kind: MessageType::Message,
                ..Default::default()
            })
            .collect();
        channel.chan.total_messages = 2200;
    }
    state.apply(ServerEvent::More(MoreEvent {
        chan: 2,
        messages: (-99..=0)
            .map(|id| Message {
                id,
                text: format!("older {id}"),
                kind: MessageType::Message,
                ..Default::default()
            })
            .collect(),
        total_messages: 2200,
    }));
    let channel = state.find_channel(2).unwrap().1;
    assert_eq!(channel.chan.messages.len(), 2100);
    assert_eq!(channel.chan.messages.first().unwrap().id, -99);
}

#[test]
fn part_removes_user_from_channel_list() {
    let mut state = init_state();
    state.apply(ServerEvent::Names(NamesEvent {
        id: 2,
        users: vec![
            User {
                nick: "alice".into(),
                modes: vec!["o".into()],
                ..Default::default()
            },
            User {
                nick: "bob".into(),
                modes: vec!["v".into()],
                ..Default::default()
            },
        ],
    }));
    state.apply(ServerEvent::Message(Box::new(MsgEvent {
        chan: 2,
        msg: Message {
            id: 12,
            kind: MessageType::Part,
            from: Some(UserInMessage {
                nick: "alice".into(),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    })));
    let users = &state.find_channel(2).unwrap().1.users;
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].nick, "bob");
}

#[test]
fn open_event_clears_unread_on_other_channels() {
    let mut state = init_state();
    state.set_active(1);
    state.apply(ServerEvent::Open(2));
    let channel = state.find_channel(2).unwrap().1;
    assert_eq!(channel.chan.unread, 0);
    assert_eq!(channel.chan.highlight, 0);
}
