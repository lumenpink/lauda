use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealtimeEvent {
    Connected,
    Disconnected,
    MailboxUpdated { mailbox: String },
    MessageAction { action: String, uid: Option<u32> },
}

#[derive(Debug, Deserialize)]
struct RawRealtimePayload {
    event: Option<String>,
    mailbox: Option<String>,
    r#box: Option<String>,
    uid: Option<u32>,
}

/// Converte um payload textual de evento JSON vindo do servidor em `RealtimeEvent`.
pub fn parse_server_realtime_event(payload: &str) -> Option<RealtimeEvent> {
    let raw: RawRealtimePayload = serde_json::from_str(payload).ok()?;
    let event = raw.event?;

    match event.as_str() {
        "mailbox_updated" => {
            let mb = raw.mailbox.or(raw.r#box).unwrap_or_else(|| "INBOX".to_string());
            Some(RealtimeEvent::MailboxUpdated { mailbox: mb })
        }
        "read_update" | "flag_update" | "delete" | "move" => Some(RealtimeEvent::MessageAction {
            action: event,
            uid: raw.uid,
        }),
        _ => None,
    }
}

/// Constrói a URL canônica do WebSocket `/ws?token={token}` a partir da URL HTTP do servidor.
pub fn build_ws_url(server_url: &str, token: &str) -> String {
    let base = server_url.trim_end_matches('/');
    let ws_base = if base.starts_with("https://") {
        base.replacen("https://", "wss://", 1)
    } else if base.starts_with("http://") {
        base.replacen("http://", "ws://", 1)
    } else {
        format!("ws://{}", base)
    };
    format!("{}/ws?token={}", ws_base, token)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn run_desktop_realtime_loop(
    ws_url: String,
    tx: tokio::sync::mpsc::UnboundedSender<RealtimeEvent>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let mut backoff_secs = 1u64;

    while !*shutdown.borrow() {
        match tokio_tungstenite::connect_async(&ws_url).await {
            Ok((mut socket, _)) => {
                backoff_secs = 1;
                let _ = tx.send(RealtimeEvent::Connected);

                let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(20));
                heartbeat.tick().await;

                loop {
                    tokio::select! {
                        _ = shutdown.changed() => {
                            if *shutdown.borrow() {
                                let _ = socket.close(None).await;
                                return;
                            }
                        }
                        _ = heartbeat.tick() => {
                            if socket.send(Message::Ping(Default::default())).await.is_err() {
                                break;
                            }
                        }
                        msg_opt = socket.next() => {
                            match msg_opt {
                                Some(Ok(Message::Text(text))) => {
                                    if let Some(event) = parse_server_realtime_event(&text) {
                                        let _ = tx.send(event);
                                    }
                                }
                                Some(Ok(Message::Ping(payload))) => {
                                    let _ = socket.send(Message::Pong(payload)).await;
                                }
                                Some(Ok(Message::Pong(_))) => {}
                                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => {
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }

                let _ = tx.send(RealtimeEvent::Disconnected);
            }
            Err(_e) => {
                let _ = tx.send(RealtimeEvent::Disconnected);
            }
        }

        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    return;
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)) => {
                backoff_secs = (backoff_secs * 2).min(30);
            }
        }
    }
}
