use dioxus::prelude::*;
use lauda_types::{ComposeForm, MessageEnvelope};

use crate::adapters::HttpApiClient;
use crate::domain::ports::MailApiClient;
use crate::domain::session::SessionConfig;
use crate::ui::state::Screen;

#[component]
pub fn ComposeView(
    screen: Signal<Screen>,
    session: Signal<SessionConfig>,
    reply_to: Option<String>,
    reply_cc: Option<String>,
    reply_subject: Option<String>,
    initial_body: Option<String>,
    current_mailbox: Signal<String>,
    messages: Signal<Vec<MessageEnvelope>>,
) -> Element {
    let sess = session.read().clone();
    let sender_address = if sess.username.contains('@') {
        sess.username.clone()
    } else {
        format!("{}@2lp.in", sess.username)
    };

    let mut from_input = use_signal(|| sender_address);
    let mut to_input = use_signal(|| reply_to.unwrap_or_default());
    let mut cc_input = use_signal(|| reply_cc.unwrap_or_default());
    let mut subject_input = use_signal(|| reply_subject.unwrap_or_default());
    let mut body_input = use_signal(|| initial_body.unwrap_or_default());
    let mut attached_files = use_signal(Vec::<lauda_types::AttachmentUpload>::new);
    let mut to_suggestions = use_signal(Vec::<lauda_types::contact::ContactEntry>::new);

    let is_sending = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    let handle_send = move |_| {
        let to = to_input.read().trim().to_string();
        let subject = subject_input.read().trim().to_string();
        let from = from_input.read().trim().to_string();
        let cc_val = cc_input.read().trim().to_string();
        let cc = if cc_val.is_empty() {
            None
        } else {
            Some(cc_val)
        };
        let body = body_input.read().clone();
        let attachments = attached_files.read().clone();

        if to.is_empty() {
            error_msg.set(Some("Destinatário (Para) é obrigatório.".to_string()));
            return;
        }

        if subject.is_empty() {
            error_msg.set(Some("Assunto é obrigatório.".to_string()));
            return;
        }

        let sess_snapshot = session.read().clone();
        if sess_snapshot.token.is_some() {
            let form = ComposeForm {
                from,
                to,
                cc,
                subject,
                body,
                attachments,
            };

            // Enfileirar optimistamente na Outbox local
            let cache_dir = crate::domain::cache::get_default_cache_dir();
            let queue = crate::domain::outbox::OutboxQueue::new(&cache_dir);
            let now_unix = crate::domain::time::current_unix_timestamp();

            if let Err(err) = queue.enqueue(form.clone(), now_unix) {
                error_msg.set(Some(format!(
                    "Falha ao enfileirar mensagem na Outbox: {}",
                    err
                )));
                return;
            }

            // Fechar tela de composição imediatamente (Optimistic UI - 0ms de bloqueio)
            screen.set(Screen::Inbox);

            // Disparar envio em segundo plano
            if let Some(token) = sess_snapshot.token {
                let box_name = current_mailbox.read().clone();
                spawn(async move {
                    let client = HttpApiClient::new(sess_snapshot.server_url.clone());
                    let items = queue.list().unwrap_or_default();
                    for item in items {
                        let _ = queue.update_status(
                            &item.id,
                            crate::domain::outbox::OutboxStatus::Sending,
                            false,
                        );
                        match client.send_message(&token, &item.form).await {
                            Ok(_) => {
                                let _ = queue.remove(&item.id);
                                if let Ok(msgs) = client.list_messages(&token, &box_name, Some(50), Some(0)).await {
                                    messages.set(msgs);
                                }
                            }
                            Err(err) => {
                                let _ = queue.update_status(
                                    &item.id,
                                    crate::domain::outbox::OutboxStatus::Failed(err.to_string()),
                                    true,
                                );
                            }
                        }
                    }
                });
            }
        }
    };

    rsx! {
        div {
            style: "border: 2px solid #0a0a0a; background: #fff; padding: 1.5rem; box-shadow: 6px 6px 0 #0a0a0a; display: flex; flex-direction: column; gap: 1rem; max-width: 680px; margin: 0 auto;",

            // Barra de Navegação do Compositor
            div {
                style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 2px solid #0a0a0a; padding-bottom: 0.8rem;",
                div {
                    span {
                        style: "background: #dc143c; color: #fff; padding: 0.2rem 0.5rem; font-weight: bold; font-size: 0.8rem;",
                        "REDAÇÃO RÁPIDA"
                    }
                    h2 {
                        style: "font-size: 1.3rem; margin: 0.3rem 0 0 0; color: #0a0a0a;",
                        "NOVA MENSAGEM"
                    }
                }
                div {
                    style: "display: flex; gap: 0.5rem;",
                    button {
                        style: "background: #fff; color: #0a0a0a; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer;",
                        onclick: move |_| screen.set(Screen::Inbox),
                        "← CANCELAR"
                    }
                    button {
                        style: if *is_sending.read() { "background: #666; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: not-allowed;" } else { "background: #dc143c; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;" },
                        disabled: *is_sending.read(),
                        onclick: handle_send,
                        if *is_sending.read() { "ENVIANDO..." } else { "ENVIAR ✉" }
                    }
                }
            }

            if let Some(err) = error_msg.read().as_ref() {
                div {
                    style: "border: 2px solid #dc143c; background: #ffebee; color: #b71c1c; padding: 0.6rem; font-weight: bold; font-size: 0.85rem;",
                    "⚠ {err}"
                }
            }

            div {
                style: "display: flex; flex-direction: column; gap: 0.8rem;",
                div {
                    label { style: "display: block; font-weight: bold; font-size: 0.8rem; margin-bottom: 0.2rem;", "DE:" }
                    input {
                        style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 1px solid #0a0a0a; font-family: monospace;",
                        value: "{from_input}",
                        oninput: move |evt| from_input.set(evt.value()),
                    }
                }
                div {
                    label { style: "display: block; font-weight: bold; font-size: 0.8rem; margin-bottom: 0.2rem;", "PARA:" }
                    input {
                        style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 2px solid #0a0a0a; font-family: monospace;",
                        value: "{to_input}",
                        oninput: move |evt| {
                            let val = evt.value();
                            to_input.set(val.clone());
                            let cache_dir = crate::domain::cache::get_default_cache_dir();
                            if let Ok(cache) = crate::domain::contacts_cache::LocalContactsCache::open(&cache_dir) {
                                if !val.trim().is_empty() {
                                    let matches = cache.search_contacts(val.trim(), 5).unwrap_or_default();
                                    to_suggestions.set(matches);
                                } else {
                                    to_suggestions.set(Vec::new());
                                }
                            }
                        },
                        placeholder: "destinatario@dominio.com",
                    }
                    if !to_suggestions.read().is_empty() {
                        div {
                            style: "border: 2px solid #0a0a0a; background: #fff; box-shadow: 4px 4px 0 #0a0a0a; display: flex; flex-direction: column; margin-top: 0.2rem; max-height: 180px; overflow-y: auto;",
                            for c in to_suggestions.read().clone() {
                                {
                                    let c_id = c.id.clone();
                                    let name = c.display_name.clone();
                                    let email = c.primary_email.clone();
                                    let email_display = format!("<{email}>");
                                    let has_auto = c.has_autocrypt;
                                    let recipient = crate::domain::contacts_cache::format_contact_recipient(name.as_deref(), &email);
                                    rsx! {
                                        button {
                                            key: "{c_id}",
                                            style: "background: #fff; border: none; border-bottom: 1px solid #ddd; padding: 0.5rem 0.8rem; text-align: left; cursor: pointer; display: flex; justify-content: space-between; align-items: center;",
                                            onclick: move |_| {
                                                to_input.set(recipient.clone());
                                                to_suggestions.set(Vec::new());
                                            },
                                            div {
                                                style: "display: flex; gap: 0.5rem; align-items: center;",
                                                if let Some(ref n) = name {
                                                    span { style: "font-weight: bold; color: #0a0a0a;", "{n}" }
                                                }
                                                span { style: "font-family: monospace; color: #555; font-size: 0.85rem;", "{email_display}" }
                                            }
                                            if has_auto {
                                                span {
                                                    style: "background: #2e8b57; color: #fff; font-size: 0.75rem; padding: 0.15rem 0.4rem; font-weight: bold;",
                                                    "🔒 PGP"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    label { style: "display: block; font-weight: bold; font-size: 0.8rem; margin-bottom: 0.2rem;", "CC (OPCIONAL):" }
                    input {
                        style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 1px solid #0a0a0a; font-family: monospace;",
                        value: "{cc_input}",
                        oninput: move |evt| cc_input.set(evt.value()),
                    }
                }
                div {
                    label { style: "display: block; font-weight: bold; font-size: 0.8rem; margin-bottom: 0.2rem;", "ASSUNTO:" }
                    input {
                        style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 2px solid #0a0a0a; font-family: monospace;",
                        value: "{subject_input}",
                        oninput: move |evt| subject_input.set(evt.value()),
                        placeholder: "Assunto da mensagem",
                    }
                }
                div {
                    label { style: "display: block; font-weight: bold; font-size: 0.8rem; margin-bottom: 0.2rem;", "MENSAGEM (TEXTO PURO / MARKDOWN):" }
                    textarea {
                        style: "width: 100%; box-sizing: border-box; padding: 0.6rem; border: 2px solid #0a0a0a; font-family: monospace; min-height: 180px; font-size: 0.95rem; line-height: 1.4;",
                        value: "{body_input}",
                        oninput: move |evt| body_input.set(evt.value()),
                        placeholder: "Escreva sua mensagem aqui...",
                    }
                }

                // Seção de Anexos
                div {
                    style: "border-top: 1px dashed #0a0a0a; padding-top: 0.8rem; display: flex; flex-direction: column; gap: 0.5rem;",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center;",
                        span { style: "font-weight: bold; font-size: 0.85rem; color: #0a0a0a;", "📎 ANEXOS ({attached_files.read().len()}):" }
                    }

                    if !attached_files.read().is_empty() {
                        div {
                            style: "display: flex; flex-direction: column; gap: 0.4rem;",
                            for (idx, att) in attached_files.read().iter().enumerate() {
                                div {
                                    key: "{idx}",
                                    style: "display: flex; justify-content: space-between; align-items: center; background: #faf9f6; border: 1px solid #0a0a0a; padding: 0.3rem 0.6rem;",
                                    span { style: "font-size: 0.85rem; font-family: monospace; font-weight: bold;", "{att.filename}" }
                                    button {
                                        style: "background: #dc143c; color: #fff; border: none; padding: 0.2rem 0.5rem; font-size: 0.75rem; font-weight: bold; cursor: pointer;",
                                        onclick: move |_| {
                                            attached_files.write().remove(idx);
                                        },
                                        "REMOVER ✕"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
