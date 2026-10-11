use dioxus::prelude::*;
use lauda_types::{MailboxInfo, Message, MessageEnvelope, ViewMode};

use crate::adapters::HttpApiClient;
use crate::domain::ports::MailApiClient;
use crate::domain::session::SessionConfig;
use crate::domain::view::{clean_comment_tags, parse_markdown_to_html};
use crate::ui::state::Screen;

#[component]
pub fn MessageView(
    mut screen: Signal<Screen>,
    session: Signal<SessionConfig>,
    current_mailbox: Signal<String>,
    mut messages: Signal<Vec<MessageEnvelope>>,
    mut mailboxes: Signal<Vec<MailboxInfo>>,
    mut active_message: Signal<Option<Message>>,
    mut current_view_mode: Signal<ViewMode>,
) -> Element {
    let msg = active_message.read().clone();
    let mut download_status = use_signal(|| None::<(usize, String)>);

    if msg.is_none() {
        return rsx! {
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 2rem; text-align: center;",
                p { "Nenhuma mensagem carregada." }
                button {
                    style: "background: #0a0a0a; color: #fff; padding: 0.5rem 1rem; border: none; cursor: pointer;",
                    onclick: move |_| screen.set(Screen::Inbox),
                    "← VOLTAR À CAIXA DE ENTRADA"
                }
            }
        };
    }

    let message = msg.unwrap();
    let envelope = message.envelope.clone();
    let sender_for_reply = envelope.sender.clone();
    let cc_for_reply = envelope.cc.clone();
    let subject_for_reply = if envelope.subject.starts_with("Re:") {
        envelope.subject.clone()
    } else {
        format!("Re: {}", envelope.subject)
    };
    let subject_for_forward = if envelope.subject.starts_with("Fwd:") {
        envelope.subject.clone()
    } else {
        format!("Fwd: {}", envelope.subject)
    };

    let raw_quote_source = message
        .txt_original
        .as_deref()
        .or(message.txt_auto.as_deref())
        .unwrap_or("");
    let body_quote_source = clean_comment_tags(raw_quote_source);
    let quoted_lines = body_quote_source
        .lines()
        .map(|l| format!("> {}", l))
        .collect::<Vec<_>>()
        .join("\n");
    let reply_body = format!(
        "\n\nEm {}, {} escreveu:\n{}",
        envelope.date, envelope.sender, quoted_lines
    );
    let forward_body = format!(
        "\n\n---------- Mensagem Encaminhada ----------\nDe: {}\nData: {}\nAssunto: {}\n\n{}",
        envelope.sender, envelope.date, envelope.subject, body_quote_source
    );

    let handle_toggle_read = move |_| {
        let sess = session.read().clone();
        let box_name = current_mailbox.read().clone();
        if let Some(token) = sess.token
            && let Some(current) = active_message.read().clone()
        {
            let uid = current.envelope.uid;
            let new_read = !current.envelope.is_read;
            spawn(async move {
                let client = HttpApiClient::new(sess.server_url);
                if client
                    .mark_read(&token, &box_name, uid, new_read)
                    .await
                    .is_ok()
                {
                    if let Some(m) = active_message.write().as_mut() {
                        m.envelope.is_read = new_read;
                    }
                    if let Some(m) = messages.write().iter_mut().find(|m| m.uid == uid) {
                        m.is_read = new_read;
                    }
                }
            });
        }
    };

    let handle_toggle_flag = move |_| {
        let sess = session.read().clone();
        let box_name = current_mailbox.read().clone();
        if let Some(token) = sess.token
            && let Some(current) = active_message.read().clone()
        {
            let uid = current.envelope.uid;
            let new_flagged = !current.envelope.is_flagged;
            spawn(async move {
                let client = HttpApiClient::new(sess.server_url);
                if client
                    .mark_flagged(&token, &box_name, uid, new_flagged)
                    .await
                    .is_ok()
                {
                    if let Some(m) = active_message.write().as_mut() {
                        m.envelope.is_flagged = new_flagged;
                    }
                    if let Some(m) = messages.write().iter_mut().find(|m| m.uid == uid) {
                        m.is_flagged = new_flagged;
                    }
                }
            });
        }
    };

    let handle_archive = move |_| {
        let sess = session.read().clone();
        let box_name = current_mailbox.read().clone();
        if let Some(token) = sess.token
            && let Some(current) = active_message.read().clone()
        {
            let uid = current.envelope.uid;
            spawn(async move {
                let client = HttpApiClient::new(sess.server_url.clone());
                if client
                    .move_message(&token, uid, &box_name, "Archive")
                    .await
                    .is_ok()
                {
                    messages.write().retain(|m| m.uid != uid);
                    active_message.set(None);
                    if let Ok(boxes) = client.list_mailboxes(&token).await {
                        mailboxes.set(boxes);
                    }
                    screen.set(Screen::Inbox);
                }
            });
        }
    };

    let handle_delete = move |_| {
        let sess = session.read().clone();
        let box_name = current_mailbox.read().clone();
        if let Some(token) = sess.token
            && let Some(current) = active_message.read().clone()
        {
            let uid = current.envelope.uid;
            spawn(async move {
                let client = HttpApiClient::new(sess.server_url);
                if client.delete_message(&token, &box_name, uid).await.is_ok() {
                    messages.write().retain(|m| m.uid != uid);
                    active_message.set(None);
                    screen.set(Screen::Inbox);
                }
            });
        }
    };

    let raw_txt_auto = message.txt_auto.clone().unwrap_or_default();
    let raw_txt_original = message.txt_original.clone().unwrap_or_default();
    let txt_auto_body = clean_comment_tags(&raw_txt_auto);
    let txt_original_body = clean_comment_tags(&raw_txt_original);
    let html_body = message.html_sanitized.clone().unwrap_or_default();

    let rendered_markdown_html = parse_markdown_to_html(&txt_auto_body);

    let server_url = session.read().server_url.clone();
    let token_opt = session.read().token.clone();
    let current_box_name = current_mailbox.read().clone();

    let formatted_attachments: Vec<(usize, String, String, String, String)> = message
        .attachments
        .iter()
        .map(|att| {
            let size_kb = (att.size_bytes as f64 / 1024.0).max(0.1);
            let size_str = if size_kb > 1024.0 {
                format!("{:.1} MB", size_kb / 1024.0)
            } else {
                format!("{:.1} KB", size_kb)
            };
            let token_param = if let Some(ref t) = token_opt {
                format!("&token={}", t)
            } else {
                String::new()
            };
            let dl_url = format!(
                "{}/messages/{}/attachments/{}?box={}{}",
                server_url, envelope.uid, att.id, current_box_name, token_param
            );
            (
                att.id,
                att.filename.clone(),
                att.content_type.clone(),
                size_str,
                dl_url,
            )
        })
        .collect();

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 1rem;",

            // Barra de Navegação e Ações Rápidas
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 0.8rem 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 0.8rem;",
                div {
                    style: "display: flex; gap: 0.5rem; flex-wrap: wrap;",
                    button {
                        style: "background: #fff; color: #0a0a0a; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer;",
                        onclick: move |_| screen.set(Screen::Inbox),
                        "← VOLTAR"
                    }
                    button {
                        style: "background: #dc143c; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                        onclick: {
                            let s_rep = sender_for_reply.clone();
                            let sub_rep = subject_for_reply.clone();
                            let b_rep = reply_body.clone();
                            move |_| screen.set(Screen::Compose {
                                reply_to: Some(s_rep.clone()),
                                reply_cc: None,
                                reply_subject: Some(sub_rep.clone()),
                                initial_body: Some(b_rep.clone()),
                            })
                        },
                        "RESPONDER"
                    }
                    button {
                        style: "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.4rem 0.8rem; font-size: 0.85rem; font-weight: bold; cursor: pointer;",
                        onclick: {
                            let s_rep = sender_for_reply.clone();
                            let cc_rep = cc_for_reply.clone();
                            let sub_rep = subject_for_reply.clone();
                            let b_rep = reply_body.clone();
                            move |_| screen.set(Screen::Compose {
                                reply_to: Some(s_rep.clone()),
                                reply_cc: cc_rep.clone(),
                                reply_subject: Some(sub_rep.clone()),
                                initial_body: Some(b_rep.clone()),
                            })
                        },
                        "RESP. TODOS"
                    }
                    button {
                        style: "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.4rem 0.8rem; font-size: 0.85rem; font-weight: bold; cursor: pointer;",
                        onclick: {
                            let sub_fwd = subject_for_forward.clone();
                            let b_fwd = forward_body.clone();
                            move |_| screen.set(Screen::Compose {
                                reply_to: None,
                                reply_cc: None,
                                reply_subject: Some(sub_fwd.clone()),
                                initial_body: Some(b_fwd.clone()),
                            })
                        },
                        "ENCAMINHAR"
                    }
                    button {
                        style: "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.4rem 0.8rem; font-size: 0.85rem; font-weight: bold; cursor: pointer;",
                        onclick: handle_toggle_flag,
                        if envelope.is_flagged { "★ FAVORITO" } else { "☆ MARCAR" }
                    }
                    button {
                        style: "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.4rem 0.8rem; font-size: 0.85rem; font-weight: bold; cursor: pointer;",
                        onclick: handle_archive,
                        "📦 ARQUIVAR"
                    }
                    button {
                        style: "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.4rem 0.8rem; font-size: 0.85rem; font-weight: bold; cursor: pointer;",
                        onclick: handle_toggle_read,
                        if envelope.is_read { "MARCAR NÃO LIDA" } else { "MARCAR LIDA" }
                    }
                    button {
                        style: "background: #fff; color: #8b0000; border: 1px solid #8b0000; padding: 0.4rem 0.8rem; font-size: 0.85rem; font-weight: bold; cursor: pointer;",
                        onclick: handle_delete,
                        "🗑 EXCLUIR"
                    }
                }
            }

            // Metadados do Envelope
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 1.2rem; box-shadow: 4px 4px 0 #0a0a0a;",
                div {
                    style: "display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 0.5rem;",
                    h2 {
                        style: "font-size: 1.3rem; margin: 0; color: #0a0a0a; font-weight: bold;",
                        if envelope.is_flagged { "★ " } else { "" }
                        "{envelope.subject}"
                    }
                    span {
                        style: "background: #0a0a0a; color: #fff; font-size: 0.75rem; padding: 0.2rem 0.4rem;",
                        "UID: {envelope.uid}"
                    }
                }
                div {
                    style: "font-size: 0.9rem; color: #333; display: flex; flex-direction: column; gap: 0.2rem;",
                    div { span { style: "font-weight: bold; color: #0a0a0a;", "DE: " } "{envelope.sender}" }
                    if let Some(cc) = envelope.cc.as_ref() {
                        div { span { style: "font-weight: bold; color: #0a0a0a;", "CC: " } "{cc}" }
                    }
                    div { span { style: "font-weight: bold; color: #0a0a0a;", "DATA: " } "{envelope.date}" }
                }
            }

            // Corpo da Mensagem conforme o Modo Trifocal
            {
                let is_html_mode = *current_view_mode.read() == ViewMode::Html;
                let container_style = if is_html_mode {
                    "border: 2px solid #0a0a0a; background: #fff; padding: 0; box-shadow: 6px 6px 0 #0a0a0a; overflow: hidden;"
                } else {
                    "border: 2px solid #0a0a0a; background: #fff; padding: 1.5rem; box-shadow: 6px 6px 0 #0a0a0a; min-height: 250px;"
                };

                rsx! {
                    div {
                        style: "{container_style}",
                        match *current_view_mode.read() {
                            ViewMode::TxtAuto => rsx! {
                                div {
                                    style: "line-height: 1.6; font-size: 1rem; color: #0a0a0a;",
                                    dangerous_inner_html: "{rendered_markdown_html}",
                                }
                            },
                            ViewMode::TxtOriginal => rsx! {
                                pre {
                                    style: "white-space: pre-wrap; font-family: monospace; font-size: 0.95rem; line-height: 1.5; color: #0a0a0a; margin: 0; background: #faf9f6; padding: 1rem; border: 1px dashed #0a0a0a;",
                                    "{txt_original_body}"
                                }
                            },
                            ViewMode::Html => rsx! {
                                div {
                                    style: "line-height: 1.5; font-size: 0.95rem; color: #0a0a0a; background: transparent; padding: 0; overflow-x: auto; width: 100%;",
                                    dangerous_inner_html: "{html_body}",
                                }
                            },
                        }
                    }
                }
            }

            // Bloco de Anexos
            if !message.attachments.is_empty() {
                div {
                    style: "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 4px 4px 0 #0a0a0a;",
                    div {
                        style: "font-weight: bold; font-size: 0.9rem; color: #0a0a0a; margin-bottom: 0.8rem; border-bottom: 1px solid #0a0a0a; padding-bottom: 0.4rem; display: flex; align-items: center; gap: 0.4rem;",
                        span { "📎 ANEXOS (" }
                        span { style: "color: #dc143c;", "{message.attachments.len()}" }
                        span { ")" }
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 0.5rem;",
                        for (id, filename, content_type, size_str, dl_url) in &formatted_attachments {
                            {
                                let att_id = *id;
                                let fname = filename.clone();
                                let box_name = current_mailbox.read().clone();
                                let uid = envelope.uid;
                                let sess = session.read().clone();
                                let status_for_this = download_status.read().clone();

                                rsx! {
                                    div {
                                        key: "{id}",
                                        style: "display: flex; flex-direction: column; background: #faf9f6; border: 1px solid #0a0a0a; padding: 0.5rem 0.8rem; gap: 0.4rem;",
                                        div {
                                            style: "display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; flex-wrap: wrap;",
                                            div {
                                                style: "display: flex; flex-direction: column;",
                                                span { style: "font-weight: bold; font-size: 0.9rem; color: #0a0a0a;", "{filename}" }
                                                span { style: "font-size: 0.75rem; color: #666;", "{content_type} • {size_str}" }
                                            }
                                            div {
                                                style: "display: flex; gap: 0.4rem; align-items: center;",
                                                button {
                                                    style: "background: #fff; color: #0a0a0a; padding: 0.3rem 0.6rem; font-size: 0.8rem; font-weight: bold; border: 1px solid #0a0a0a; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                                                    onclick: move |_| {
                                                        let sess = sess.clone();
                                                        let box_name = box_name.clone();
                                                        let fname = fname.clone();
                                                        download_status.set(Some((att_id, "Baixando...".to_string())));
                                                        if let Some(token) = sess.token {
                                                            spawn(async move {
                                                                let client = HttpApiClient::new(sess.server_url);
                                                                match client.fetch_attachment(&token, &box_name, uid, att_id).await {
                                                                    Ok((_name, _ct, bytes)) => {
                                                                        let target = resolve_download_dir().join(&fname);
                                                                        if std::fs::write(&target, bytes).is_ok() {
                                                                            download_status.set(Some((att_id, format!("✓ Salvo em {}", target.display()))));
                                                                        } else {
                                                                            download_status.set(Some((att_id, "Erro ao gravar arquivo no disco".to_string())));
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        download_status.set(Some((att_id, format!("Falha no download: {e}"))));
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    },
                                                    "💾 SALVAR"
                                                }
                                                a {
                                                    href: "{dl_url}",
                                                    target: "_blank",
                                                    download: "{filename}",
                                                    style: "background: #0a0a0a; color: #fff; text-decoration: none; padding: 0.3rem 0.6rem; font-size: 0.8rem; font-weight: bold; border: 1px solid #0a0a0a; box-shadow: 2px 2px 0 #dc143c; white-space: nowrap;",
                                                    "LINK"
                                                }
                                            }
                                        }
                                        if let Some((sid, msg)) = status_for_this {
                                            if sid == att_id {
                                                div {
                                                    style: "font-size: 0.75rem; color: #dc143c; font-weight: bold; background: #fff; padding: 0.2rem 0.4rem; border-left: 2px solid #dc143c;",
                                                    "{msg}"
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
        }
    }
}

fn resolve_download_dir() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let p = std::path::PathBuf::from(home).join("Downloads");
        if p.exists() {
            return p;
        }
    }
    std::env::temp_dir()
}
