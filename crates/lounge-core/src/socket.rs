use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::proto::{InitData, ServerEvent, socket_url};

const PING_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone)]
pub enum Auth {
    Password { user: String, password: String },
    Token { user: String, token: String },

    Public,
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub auth: Auth,
    pub initial_last_message: i64,
    pub initial_open_channel: Option<i64>,
    pub has_config: bool,
}

#[derive(Debug, Clone)]
pub enum Command {
    Emit {
        event: String,
        data: Option<Value>,
    },
    Open(i64),
    Input {
        target: i64,
        text: String,
        reply_to: Option<String>,
    },
    Names(i64),
    More {
        target: i64,
        last_id: i64,
        condensed: bool,
    },
    Close,
}

#[derive(Debug, Clone)]
pub enum ClientEvent {
    Connecting,
    Handshake,
    Authenticating,
    Authenticated,
    AuthFailed,
    Init(Box<InitData>),
    Server(Box<ServerEvent>),
    Disconnected { reason: String },
    Reconnecting { attempt: u32, delay: Duration },
    Error(String),
    Closed,
}

#[derive(Clone)]
pub struct ClientHandle {
    tx: mpsc::UnboundedSender<Command>,
}

impl ClientHandle {
    pub fn emit(&self, event: impl Into<String>, data: Option<Value>) {
        let _ = self.tx.send(Command::Emit {
            event: event.into(),
            data,
        });
    }

    pub fn open(&self, id: i64) {
        let _ = self.tx.send(Command::Open(id));
    }

    pub fn input(&self, target: i64, text: impl Into<String>, reply_to: Option<String>) {
        let _ = self.tx.send(Command::Input {
            target,
            text: text.into(),
            reply_to,
        });
    }

    pub fn names(&self, target: i64) {
        let _ = self.tx.send(Command::Names(target));
    }

    pub fn more(&self, target: i64, last_id: i64, condensed: bool) {
        let _ = self.tx.send(Command::More {
            target,
            last_id,
            condensed,
        });
    }

    pub fn close(&self) {
        let _ = self.tx.send(Command::Close);
    }

    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

pub fn connect(config: ClientConfig) -> (ClientHandle, mpsc::UnboundedReceiver<ClientEvent>) {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (evt_tx, evt_rx) = mpsc::unbounded_channel();
    let handle = ClientHandle { tx: cmd_tx.clone() };

    std::thread::Builder::new()
        .name("lounge-socket".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = evt_tx.send(ClientEvent::Error(format!(
                        "failed to start network runtime: {error}"
                    )));
                    return;
                }
            };
            runtime.block_on(run(config, cmd_rx, evt_tx, cmd_tx));
        })
        .expect("failed to spawn socket thread");

    (handle, evt_rx)
}

enum SessionEnd {
    AuthFailed,
    Closed,
    Disconnected(String),
}

async fn run(
    config: ClientConfig,
    mut cmd_rx: mpsc::UnboundedReceiver<Command>,
    evt_tx: mpsc::UnboundedSender<ClientEvent>,
    cmd_tx: mpsc::UnboundedSender<Command>,
) {
    let mut token = match &config.auth {
        Auth::Token { token, .. } => Some(token.clone()),
        _ => None,
    };
    let mut last_message = config.initial_last_message;
    let mut open_channel = config.initial_open_channel;
    let mut attempt: u32 = 0;

    loop {
        let _ = evt_tx.send(ClientEvent::Connecting);
        match session(
            &config,
            &mut token,
            &mut last_message,
            &mut open_channel,
            &mut cmd_rx,
            &evt_tx,
            &mut attempt,
        )
        .await
        {
            SessionEnd::AuthFailed => {
                let _ = evt_tx.send(ClientEvent::AuthFailed);
                break;
            }
            SessionEnd::Closed => {
                let _ = evt_tx.send(ClientEvent::Closed);
                break;
            }
            SessionEnd::Disconnected(reason) => {
                let _ = evt_tx.send(ClientEvent::Disconnected {
                    reason: reason.clone(),
                });
                attempt = attempt.saturating_add(1);
                let delay = Duration::from_secs(match attempt {
                    1 => 1,
                    2 => 2,
                    3 => 4,
                    4 => 8,
                    _ => 15,
                });
                let _ = evt_tx.send(ClientEvent::Reconnecting { attempt, delay });
                let sleep = tokio::time::sleep(delay);
                tokio::pin!(sleep);
                loop {
                    tokio::select! {
                        _ = &mut sleep => break,
                        command = cmd_rx.recv() => match command {
                            Some(Command::Close) | None => {
                                let _ = evt_tx.send(ClientEvent::Closed);
                                return;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    drop(cmd_tx);
}

#[allow(clippy::too_many_arguments)]
async fn session(
    config: &ClientConfig,
    token: &mut Option<String>,
    last_message: &mut i64,
    open_channel: &mut Option<i64>,
    cmd_rx: &mut mpsc::UnboundedReceiver<Command>,
    evt_tx: &mpsc::UnboundedSender<ClientEvent>,
    attempt: &mut u32,
) -> SessionEnd {
    let url = match socket_url(&config.base_url) {
        Ok(url) => url,
        Err(error) => return SessionEnd::Disconnected(format!("invalid server URL: {error}")),
    };

    let (ws, _) = match tokio_tungstenite::connect_async(url.as_str()).await {
        Ok(connection) => connection,
        Err(error) => return SessionEnd::Disconnected(format!("connection failed: {error}")),
    };
    let (mut sink, mut stream) = ws.split();

    let mut received_open = false;
    while !received_open {
        match tokio::time::timeout(PING_TIMEOUT, stream.next()).await {
            Ok(Some(Ok(WsMessage::Text(text)))) => {
                let text = text.to_string();
                for packet in split_packets(&text) {
                    if packet.starts_with('0') {
                        received_open = true;
                    }
                }
            }
            Ok(Some(Ok(WsMessage::Ping(payload)))) => {
                if sink.send(WsMessage::Pong(payload)).await.is_err() {
                    return SessionEnd::Disconnected("connection lost during handshake".into());
                }
            }
            Ok(Some(Ok(WsMessage::Close(_)))) | Ok(None) => {
                return SessionEnd::Disconnected("connection closed during handshake".into());
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(error))) => {
                return SessionEnd::Disconnected(format!("websocket error: {error}"));
            }
            Err(_) => return SessionEnd::Disconnected("handshake timed out".into()),
        }
    }

    if sink.send(WsMessage::Text("40".into())).await.is_err() {
        return SessionEnd::Disconnected("failed to open socket.io namespace".into());
    }
    let _ = evt_tx.send(ClientEvent::Handshake);

    let mut auth_sent = false;
    let mut signed_out = false;

    loop {
        let sleep = tokio::time::sleep(PING_TIMEOUT);
        tokio::pin!(sleep);

        tokio::select! {
            biased;
            _ = &mut sleep => {
                return SessionEnd::Disconnected("server ping timed out".into());
            }
            command = cmd_rx.recv() => {
                let Some(command) = command else {
                    let _ = sink.send(WsMessage::Text("41".into())).await;
                    return SessionEnd::Closed;
                };
                match command {
                    Command::Close => {
                        let _ = sink.send(WsMessage::Text("41".into())).await;
                        return SessionEnd::Closed;
                    }
                    Command::Open(id) => {
                        *open_channel = Some(id);
                        if send_event(&mut sink, "open", Some(json!(id))).await.is_err() {
                            return SessionEnd::Disconnected("connection lost".into());
                        }
                    }
                    Command::Emit { event, data } => {
                        if send_event(&mut sink, &event, data).await.is_err() {
                            return SessionEnd::Disconnected("connection lost".into());
                        }
                    }
                    Command::Input { target, text, reply_to } => {
                        let mut data = json!({"target": target, "text": text});
                        if let Some(reply_to) = reply_to {
                            data["replyTo"] = json!(reply_to);
                        }
                        if send_event(&mut sink, "input", Some(data)).await.is_err() {
                            return SessionEnd::Disconnected("connection lost".into());
                        }
                    }
                    Command::Names(target) => {
                        if send_event(&mut sink, "names", Some(json!({"target": target}))).await.is_err() {
                            return SessionEnd::Disconnected("connection lost".into());
                        }
                    }
                    Command::More { target, last_id, condensed } => {
                        let data = json!({"target": target, "lastId": last_id, "condensed": condensed});
                        if send_event(&mut sink, "more", Some(data)).await.is_err() {
                            return SessionEnd::Disconnected("connection lost".into());
                        }
                    }
                }
            }
            message = stream.next() => {
                let Some(message) = message else {
                    return SessionEnd::Disconnected("connection closed".into());
                };
                let text = match message {
                    Ok(WsMessage::Text(text)) => text.to_string(),
                    Ok(WsMessage::Binary(binary)) => match String::from_utf8(binary.to_vec()) {
                        Ok(text) => text,
                        Err(_) => continue,
                    },
                    Ok(WsMessage::Ping(payload)) => {
                        if sink.send(WsMessage::Pong(payload)).await.is_err() {
                            return SessionEnd::Disconnected("connection lost".into());
                        }
                        continue;
                    }
                    Ok(WsMessage::Close(_)) => {
                        return if signed_out {
                            SessionEnd::Closed
                        } else {
                            SessionEnd::Disconnected("server closed the connection".into())
                        };
                    }
                    Ok(WsMessage::Pong(_)) | Ok(WsMessage::Frame(_)) => continue,
                    Err(error) => {
                        return SessionEnd::Disconnected(format!("websocket error: {error}"));
                    }
                };

                for packet in split_packets(&text) {
                    if packet == "2" {
                        if sink.send(WsMessage::Text("3".into())).await.is_err() {
                            return SessionEnd::Disconnected("connection lost".into());
                        }
                        continue;
                    }
                    if packet == "41" {
                        return SessionEnd::Disconnected("server disconnected socket.io".into());
                    }
                    let Some(payload) = packet.strip_prefix("42") else {
                        continue;
                    };
                    let Some((event, data)) = parse_socketio_event(payload) else {
                        continue;
                    };

                    match event.as_str() {
                        "auth:start" => {
                            if auth_sent {
                                continue;
                            }
                            auth_sent = true;
                            let Some(payload) =
                                auth_payload(config, token.as_deref(), *last_message, *open_channel)
                            else {


                                return SessionEnd::AuthFailed;
                            };
                            let _ = evt_tx.send(ClientEvent::Authenticating);
                            if send_event(&mut sink, "auth:perform", Some(payload)).await.is_err() {
                                return SessionEnd::Disconnected("connection lost".into());
                            }
                        }
                        "auth:success" => {
                            let _ = evt_tx.send(ClientEvent::Authenticated);
                        }
                        "auth:failed" => {
                            return SessionEnd::AuthFailed;
                        }
                        "init" => {
                            let Some(data) = data else { continue };
                            let Ok(init) = serde_json::from_value::<InitData>(data) else {
                                continue;
                            };

                            *attempt = 0;
                            if let Some(new_token) = &init.token {
                                *token = Some(new_token.clone());
                            }
                            for network in &init.networks {
                                for channel in &network.channels {
                                    for message in &channel.messages {
                                        *last_message = (*last_message).max(message.id);
                                    }
                                }
                            }
                            if init.active > 0 {
                                *open_channel = Some(init.active);
                            }
                            let _ = evt_tx.send(ClientEvent::Init(Box::new(init)));
                        }
                        "open" => {
                            if let Some(Value::Number(id)) = &data
                                && let Some(id) = id.as_i64()
                                && id > 0
                            {
                                let _ = evt_tx.send(ClientEvent::Server(Box::new(
                                    ServerEvent::Open(id),
                                )));
                            }
                        }
                        "msg" => {
                            if let Some(msg) = data
                                .as_ref()
                                .and_then(|value| value.get("msg"))
                                .and_then(|msg| msg.get("id"))
                                .and_then(Value::as_i64)
                            {
                                *last_message = (*last_message).max(msg);
                            }
                            if let Some(event) = ServerEvent::parse("msg", data.as_ref()) {
                                let _ = evt_tx.send(ClientEvent::Server(Box::new(event)));
                            }
                        }
                        "sign-out" => {
                            signed_out = true;
                            let _ = evt_tx.send(ClientEvent::Server(Box::new(ServerEvent::SignedOut)));
                        }
                        _ => {
                            if let Some(event) = ServerEvent::parse(&event, data.as_ref()) {
                                if matches!(event, ServerEvent::AuthSuccess | ServerEvent::AuthFailed) {
                                    continue;
                                }
                                let _ = evt_tx.send(ClientEvent::Server(Box::new(event)));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn auth_payload(
    config: &ClientConfig,
    token: Option<&str>,
    last_message: i64,
    open_channel: Option<i64>,
) -> Option<Value> {
    match &config.auth {
        Auth::Public => None,
        Auth::Token {
            user,
            token: configured,
        } => {
            let token = token.or(Some(configured.as_str()));
            Some(json!({
                "user": user,
                "token": token,
                "lastMessage": last_message,
                "openChannel": open_channel,
                "hasConfig": config.has_config,
            }))
        }
        Auth::Password { user, password } => match token {
            Some(token) => Some(json!({
                "user": user,
                "token": token,
                "lastMessage": last_message,
                "openChannel": open_channel,
                "hasConfig": config.has_config,
            })),
            None => Some(json!({
                "user": user,
                "password": password,
            })),
        },
    }
}

async fn send_event<S>(
    sink: &mut S,
    event: &str,
    data: Option<Value>,
) -> Result<(), tokio_tungstenite::tungstenite::Error>
where
    S: futures_util::Sink<WsMessage, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let packet = match data {
        Some(data) => json!([event, data]),
        None => json!([event]),
    };
    let text = format!("42{packet}");
    sink.send(WsMessage::Text(text.into())).await
}

fn split_packets(text: &str) -> Vec<&str> {
    text.split('\u{1e}')
        .filter(|part| !part.is_empty())
        .collect()
}

fn parse_socketio_event(payload: &str) -> Option<(String, Option<Value>)> {
    let start = payload.find('[')?;

    if !payload[..start].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let values: Vec<Value> = serde_json::from_str(&payload[start..]).ok()?;
    let mut values = values.into_iter();
    let event = values.next()?.as_str()?.to_string();
    Some((event, values.next()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_events() {
        let (event, data) = parse_socketio_event(r#"["init",{"active":1,"networks":[]}]"#).unwrap();
        assert_eq!(event, "init");
        assert_eq!(data.unwrap()["active"], 1);

        let (event, data) = parse_socketio_event(r#"["auth:start",12345]"#).unwrap();
        assert_eq!(event, "auth:start");
        assert_eq!(data.unwrap(), 12345);

        let (event, data) = parse_socketio_event(r#"["auth:success"]"#).unwrap();
        assert_eq!(event, "auth:success");
        assert!(data.is_none());

        let (event, _) = parse_socketio_event(r#"7["open",3]"#).unwrap();
        assert_eq!(event, "open");
        assert!(parse_socketio_event("garbage").is_none());
    }

    #[test]
    fn splits_engine_packets() {
        assert_eq!(split_packets("2\u{1e}42[\"a\"]"), vec!["2", "42[\"a\"]"]);
        assert!(split_packets("").is_empty());
    }
}
