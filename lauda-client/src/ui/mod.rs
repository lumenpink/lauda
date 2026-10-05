use dioxus::prelude::*;
use lauda_types::{MailboxInfo, Message, MessageEnvelope, ViewMode};

pub mod compose_view;
pub mod inbox_view;
pub mod login_view;
pub mod message_view;
pub mod state;

use compose_view::ComposeView;
use inbox_view::InboxView;
use login_view::LoginView;
use message_view::MessageView;
pub use state::Screen;

use crate::adapters::HttpApiClient;
use crate::domain::ports::MailApiClient;
use crate::domain::session::load_session;

/// Componente raiz do smolclient Lauda com estética Trash Polka e navegação mobile-first.
#[component]
pub fn App() -> Element {
    let initial_session = load_session(None).unwrap_or_default();
    let initial_screen = if initial_session.token.is_some() {
        Screen::Inbox
    } else {
        Screen::Login
    };

    let mut screen = use_signal(|| initial_screen);
    let session = use_signal(|| initial_session);
    let mut mailboxes = use_signal(Vec::<MailboxInfo>::new);
    let current_mailbox = use_signal(|| "INBOX".to_string());
    let mut messages = use_signal(Vec::<MessageEnvelope>::new);
    let active_message = use_signal(|| None::<Message>);
    let mut current_view_mode = use_signal(|| ViewMode::TxtAuto);

    let mut update_notice = use_signal(|| None::<crate::domain::update::UpdateManifest>);

    // Carregamento inicial assíncrono se houver sessão gravada
    use_effect(move || {
        let sess = session.read().clone();
        if let Some(token) = sess.token {
            spawn(async move {
                let client = HttpApiClient::new(sess.server_url);
                if let Ok(boxes) = client.list_mailboxes(&token).await {
                    mailboxes.set(boxes);
                }
                if let Ok(msgs) = client.list_messages(&token, "INBOX").await {
                    messages.set(msgs);
                }
            });
        }
    });

    // Verificação de atualização assíncrona
    use_effect(move || {
        spawn(async move {
            if let Ok(manifest) =
                crate::domain::update::fetch_remote_manifest("https://lauda.2lp.in/version.json")
                    .await
                && let crate::domain::update::UpdateStatus::Available(m)
                | crate::domain::update::UpdateStatus::Required(m) =
                    crate::domain::update::check_update(env!("CARGO_PKG_VERSION"), &manifest)
            {
                update_notice.set(Some(m));
            }
        });
    });

    let active_token_exists = session.read().token.is_some();

    rsx! {
        div {
            style: "background-color: #f4efe6; color: #0a0a0a; font-family: monospace; min-height: 100vh; padding: 1rem; box-sizing: border-box;",

            // Header Principal Trash Polka com Alternador Trifocal
            header {
                style: "border-bottom: 3px solid #0a0a0a; padding-bottom: 0.8rem; margin-bottom: 1.2rem; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 0.6rem;",
                div {
                    style: "cursor: pointer;",
                    onclick: move |_| {
                        if active_token_exists {
                            screen.set(Screen::Inbox);
                        }
                    },
                    h1 {
                        style: "font-size: 2rem; margin: 0; color: #0a0a0a; display: inline-block; vertical-align: middle;",
                        "LAUDA"
                    }
                    span {
                        style: "background: #dc143c; color: #fff; padding: 0.2rem 0.5rem; font-weight: bold; font-size: 0.8rem; margin-left: 0.5rem; vertical-align: middle;",
                        "SMOLCLIENT"
                    }
                }

                // Alternador Trifásico Global (Botão Único Rotativo)
                button {
                    style: "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c; white-space: nowrap;",
                    onclick: move |_| {
                        let next_mode = current_view_mode.read().next();
                        current_view_mode.set(next_mode);
                    },
                    "{current_view_mode.read().label()}"
                }
            }

            // Banner Trash Polka de Atualização Disponível
            if let Some(manifest) = update_notice.read().clone() {
                div {
                    style: "background: #dc143c; color: #fff; padding: 0.6rem 1rem; margin-bottom: 1.2rem; display: flex; justify-content: space-between; align-items: center; font-size: 0.85rem; font-weight: bold; border: 2px solid #0a0a0a; box-shadow: 3px 3px 0 #0a0a0a; flex-wrap: wrap; gap: 0.5rem;",
                    span { "NOVA VERSÃO DISPONÍVEL: v{manifest.version}" }
                    a {
                        href: "{manifest.apk_url}",
                        target: "_blank",
                        style: "background: #fff; color: #0a0a0a; padding: 0.3rem 0.7rem; text-decoration: none; font-size: 0.8rem; font-weight: bold; border: 1px solid #0a0a0a; white-space: nowrap;",
                        "ATUALIZAR APK"
                    }
                }
            }

            // Exibição da Tela Conforme o Estado de Navegação
            main {
                match screen.read().clone() {
                    Screen::Login => rsx! {
                        LoginView {
                            screen,
                            session,
                            mailboxes,
                            current_mailbox,
                            messages,
                        }
                    },
                    Screen::Inbox => rsx! {
                        InboxView {
                            screen,
                            session,
                            mailboxes,
                            current_mailbox,
                            messages,
                            active_message,
                        }
                    },
                    Screen::MessageDetail(_uid) => rsx! {
                        MessageView {
                            screen,
                            active_message,
                            current_view_mode,
                        }
                    },
                    Screen::Compose { reply_to, reply_subject } => rsx! {
                        ComposeView {
                            screen,
                            session,
                            reply_to,
                            reply_subject,
                            current_mailbox,
                            messages,
                        }
                    },
                }
            }
        }
    }
}
