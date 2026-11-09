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
fn message_account_accepts_server_values() {
    for account in [
        serde_json::json!("boycott"),
        serde_json::json!(true),
        serde_json::Value::Null,
    ] {
        let message: Message = serde_json::from_value(serde_json::json!({
            "account": account
        }))
        .unwrap();
        assert_eq!(message.text, "");
    }
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
fn parses_camel_case_message_fields() {
    let event = ServerEvent::parse(
        "msg",
        Some(&serde_json::json!({
            "chan": 2,
            "msg": {
                "id": 7,
                "text": "reply",
                "showInActive": true,
                "ctcpMessage": "ACTION waves",
                "invitedYou": true,
                "statusmsgGroup": "@",
                "replyTo": "parent",
                "replyToNick": "alice",
                "replyToText": "hello"
            }
        })),
    )
    .unwrap();
    let ServerEvent::Message(event) = event else {
        panic!("expected message event");
    };
    assert!(event.msg.show_in_active);
    assert_eq!(event.msg.ctcp_message.as_deref(), Some("ACTION waves"));
    assert!(event.msg.invited_you);
    assert_eq!(event.msg.statusmsg_group.as_deref(), Some("@"));
    assert_eq!(event.msg.reply_to.as_deref(), Some("parent"));
    assert_eq!(event.msg.reply_to_nick.as_deref(), Some("alice"));
    assert_eq!(event.msg.reply_to_text.as_deref(), Some("hello"));
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

    let ev = ServerEvent::parse("users", Some(&serde_json::json!({"chan": 4}))).unwrap();
    assert!(matches!(ev, ServerEvent::Users(users) if users.chan == 4));
}
