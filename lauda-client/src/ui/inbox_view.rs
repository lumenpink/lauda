use dioxus::prelude::*;
use lauda_types::{MailboxInfo, Message, MessageEnvelope};

use crate::adapters::HttpApiClient;
use crate::domain::ports::MailApiClient;
use crate::domain::session::{SessionConfig, clear_session};
use crate::ui::state::Screen;

#[component]
pub fn InboxView(
    mut screen: Signal<Screen>,
    mut session: Signal<SessionConfig>,
    mut mailboxes: Signal<Vec<MailboxInfo>>,
    mut current_mailbox: Signal<String>,
    mut messages: Signal<Vec<MessageEnvelope>>,
    mut active_message: Signal<Option<Message>>,
) -> Element {
    let mut is_refreshing = use_signal(|| false);

    let handle_refresh = move |_| {
        let sess = session.read().clone();
        if let Some(token) = sess.token.clone() {
            let box_name = current_mailbox.read().clone();
            is_refreshing.set(true);

            spawn(async move {
                let client = HttpApiClient::new(sess.server_url);
                if let Ok(boxes) = client.list_mailboxes(&token).await {
                    mailboxes.set(boxes);
                }
                if let Ok(msgs) = client.list_messages(&token, &box_name).await {
                    messages.set(msgs);
                }
                is_refreshing.set(false);
            });
        }
    };

    let handle_logout = move |_| {
        let _ = clear_session(None);
        session.set(SessionConfig::default());
        mailboxes.set(Vec::new());
        messages.set(Vec::new());
        active_message.set(None);
        screen.set(Screen::Login);
    };

    let username = session.read().username.clone();

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 1rem;",

            // Barra de Controle e Ações
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 0.8rem;",
                div {
                    style: "display: flex; align-items: center; gap: 0.8rem;",
                    span {
                        style: "background: #0a0a0a; color: #fff; padding: 0.3rem 0.6rem; font-weight: bold; font-size: 0.85rem;",
                        "CONTA: {username}"
                    }
                    div {
                        style: "display: flex; gap: 0.4rem;",
                        for mb in mailboxes.read().iter() {
                            {
                                let is_active = *current_mailbox.read() == mb.name;
                                let name = mb.name.clone();
                                let label = format!("{} ({})", mb.name, mb.unread_messages);
                                rsx! {
                                    button {
                                        key: "{mb.name}",
                                        style: if is_active { "background: #dc143c; color: #fff; border: 2px solid #0a0a0a; padding: 0.3rem 0.6rem; font-weight: bold; cursor: pointer;" } else { "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.3rem 0.6rem; cursor: pointer;" },
                                        onclick: move |_| {
                                            let name = name.clone();
                                            current_mailbox.set(name.clone());
                                            let sess = session.read().clone();
                                            if let Some(token) = sess.token.clone() {
                                                spawn(async move {
                                                    let client = HttpApiClient::new(sess.server_url);
                                                    if let Ok(msgs) = client.list_messages(&token, &name).await {
                                                        messages.set(msgs);
                                                    }
                                                });
                                            }
                                        },
                                        "{label}"
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    style: "display: flex; gap: 0.5rem;",
                    button {
                        style: "background: #dc143c; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                        onclick: move |_| screen.set(Screen::Compose { reply_to: None, reply_subject: None }),
                        "+ NOVA MENSAGEM"
                    }
                    button {
                        style: "background: #fff; color: #0a0a0a; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer;",
                        onclick: handle_refresh,
                        if *is_refreshing.read() { "..." } else { "🔄" }
                    }
                    button {
                        style: "background: #fff; color: #8b0000; border: 1px solid #8b0000; padding: 0.4rem 0.8rem; font-size: 0.85rem; cursor: pointer;",
                        onclick: handle_logout,
                        "SAIR"
                    }
                }
            }

            // Lista de Mensagens
            div {
                style: "display: flex; flex-direction: column; gap: 0.6rem;",
                if messages.read().is_empty() {
                    div {
                        style: "border: 2px dashed #666; background: #fff; padding: 2rem; text-align: center; color: #555;",
                        "Nenhuma mensagem encontrada na caixa postal."
                    }
                }
                for env in messages.read().iter() {
                    {
                        let uid = env.uid;
                        let unread = !env.is_read;
                        let sender = env.sender.clone();
                        let subject = env.subject.clone();
                        let date = env.date.clone();
                        let font_weight = if unread { "bold" } else { "normal" };

                        rsx! {
                            div {
                                key: "{uid}",
                                style: if unread { "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 4px 4px 0 #dc143c; cursor: pointer; display: flex; flex-direction: column; gap: 0.3rem;" } else { "border: 1px solid #0a0a0a; background: #faf9f6; padding: 1rem; cursor: pointer; display: flex; flex-direction: column; gap: 0.3rem;" },
                                onclick: move |_| {
                                    let sess = session.read().clone();
                                    let box_name = current_mailbox.read().clone();
                                    if let Some(token) = sess.token.clone() {
                                        spawn(async move {
                                            let client = HttpApiClient::new(sess.server_url);
                                            if let Ok(msg) = client.get_message(&token, &box_name, uid).await {
                                                active_message.set(Some(msg));
                                                screen.set(Screen::MessageDetail(uid));
                                            }
                                        });
                                    }
                                },
                                div {
                                    style: "display: flex; justify-content: space-between; align-items: center;",
                                    div {
                                        style: "display: flex; align-items: center; gap: 0.5rem;",
                                        if unread {
                                            span { style: "color: #dc143c; font-size: 1.1rem;", "●" }
                                        } else {
                                            span { style: "color: #999; font-size: 1.1rem;", "○" }
                                        }
                                        span { style: "font-weight: bold; font-size: 0.95rem; color: #0a0a0a;", "{sender}" }
                                    }
                                    span { style: "font-size: 0.8rem; color: #666;", "{date}" }
                                }
                                div {
                                    style: "font-size: 1.05rem; font-weight: {font_weight}; color: #0a0a0a; margin-top: 0.2rem;",
                                    "{subject}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
