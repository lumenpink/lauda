use dioxus::prelude::*;
use lauda_types::{Message, ViewMode};

use crate::domain::view::parse_markdown_to_html;
use crate::ui::state::Screen;

#[component]
pub fn MessageView(
    screen: Signal<Screen>,
    active_message: Signal<Option<Message>>,
    current_view_mode: Signal<ViewMode>,
) -> Element {
    let msg = active_message.read().clone();

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
    let subject_for_reply = if envelope.subject.starts_with("Re:") {
        envelope.subject.clone()
    } else {
        format!("Re: {}", envelope.subject)
    };

    let txt_auto_body = message.txt_auto.clone().unwrap_or_default();
    let txt_original_body = message.txt_original.clone().unwrap_or_default();
    let html_body = message.html_sanitized.clone().unwrap_or_default();

    let rendered_markdown_html = parse_markdown_to_html(&txt_auto_body);

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 1rem;",

            // Barra de Navegação e Alternador Trifocal
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 0.8rem 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 0.8rem;",
                div {
                    style: "display: flex; gap: 0.5rem;",
                    button {
                        style: "background: #fff; color: #0a0a0a; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer;",
                        onclick: move |_| screen.set(Screen::Inbox),
                        "← VOLTAR"
                    }
                    button {
                        style: "background: #dc143c; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                        onclick: move |_| screen.set(Screen::Compose {
                            reply_to: Some(sender_for_reply.clone()),
                            reply_subject: Some(subject_for_reply.clone()),
                        }),
                        "RESPONDER"
                    }
                }

                // Alternador Trifocal no Leitor (Botão Único Rotativo)
                button {
                    style: "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c; white-space: nowrap;",
                    onclick: move |_| {
                        let next_mode = current_view_mode.read().next();
                        current_view_mode.set(next_mode);
                    },
                    "{current_view_mode.read().label()}"
                }
            }

            // Metadados do Envelope
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 1.2rem; box-shadow: 4px 4px 0 #0a0a0a;",
                div {
                    style: "display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 0.5rem;",
                    h2 {
                        style: "font-size: 1.3rem; margin: 0; color: #0a0a0a; font-weight: bold;",
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
                    div { span { style: "font-weight: bold; color: #0a0a0a;", "DATA: " } "{envelope.date}" }
                    div {
                        style: "margin-top: 0.3rem; font-size: 0.8rem; font-weight: bold; color: #8b0000;",
                        {format!("MODO ATIVO: {:?}", *current_view_mode.read())}
                    }
                }
            }

            // Corpo da Mensagem conforme o Modo Trifocal
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 1.5rem; box-shadow: 6px 6px 0 #0a0a0a; min-height: 250px;",
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
                            style: "line-height: 1.5; font-size: 0.95rem; color: #0a0a0a; background: #fff; padding: 0.5rem; overflow-x: auto;",
                            dangerous_inner_html: "{html_body}",
                        }
                    },
                }
            }
        }
    }
}
