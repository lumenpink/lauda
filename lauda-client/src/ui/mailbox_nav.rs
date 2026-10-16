use dioxus::prelude::*;
use lauda_types::MailboxInfo;

use crate::adapters::HttpApiClient;
use crate::domain::folders::build_folder_tree;
use crate::domain::ports::MailApiClient;
use crate::domain::session::SessionConfig;
use crate::ui::inbox_view::InboxFilter;

#[component]
pub fn MailboxNavigation(
    session: Signal<SessionConfig>,
    mailboxes: Signal<Vec<MailboxInfo>>,
    mut current_mailbox: Signal<String>,
    mut messages: Signal<Vec<lauda_types::MessageEnvelope>>,
    mut active_filter: Signal<InboxFilter>,
    mut is_drawer_open: Signal<bool>,
) -> Element {
    let folder_tree = build_folder_tree(&mailboxes.read());

    let mut select_mailbox = move |raw_name: String| {
        current_mailbox.set(raw_name.clone());
        is_drawer_open.set(false);

        let sess = session.read().clone();
        if let Some(token) = sess.token.clone() {
            spawn(async move {
                let client = HttpApiClient::new(sess.server_url);
                if let Ok(msgs) = client.list_messages(&token, &raw_name, Some(50), Some(0)).await {
                    messages.set(msgs);
                }
            });
        }
    };

    let active_box = current_mailbox.read().clone();
    let current_filt = *active_filter.read();

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 1rem; width: 100%;",

            // Seção: Pastas Virtuais / Filtros Rápidos
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 0.8rem; box-shadow: 3px 3px 0 #0a0a0a;",
                div {
                    style: "font-size: 0.75rem; font-weight: bold; color: #dc143c; text-transform: uppercase; margin-bottom: 0.6rem; letter-spacing: 1px; border-bottom: 1px solid #0a0a0a; padding-bottom: 0.3rem;",
                    "FILTROS RÁPIDOS"
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 0.3rem;",

                    button {
                        style: if current_filt == InboxFilter::All {
                            "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.6rem; font-size: 0.85rem; font-weight: bold; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center;"
                        } else {
                            "background: #fff; color: #0a0a0a; border: 1px solid transparent; padding: 0.4rem 0.6rem; font-size: 0.85rem; font-weight: bold; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center;"
                        },
                        onclick: move |_| {
                            active_filter.set(InboxFilter::All);
                            is_drawer_open.set(false);
                        },
                        span { "⚡ Todas" }
                    }

                    button {
                        style: if current_filt == InboxFilter::Unread {
                            "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.6rem; font-size: 0.85rem; font-weight: bold; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center;"
                        } else {
                            "background: #fff; color: #0a0a0a; border: 1px solid transparent; padding: 0.4rem 0.6rem; font-size: 0.85rem; font-weight: bold; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center;"
                        },
                        onclick: move |_| {
                            active_filter.set(InboxFilter::Unread);
                            is_drawer_open.set(false);
                        },
                        span { "✉ Não Lidas" }
                    }

                    button {
                        style: if current_filt == InboxFilter::Flagged {
                            "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.6rem; font-size: 0.85rem; font-weight: bold; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center;"
                        } else {
                            "background: #fff; color: #0a0a0a; border: 1px solid transparent; padding: 0.4rem 0.6rem; font-size: 0.85rem; font-weight: bold; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center;"
                        },
                        onclick: move |_| {
                            active_filter.set(InboxFilter::Flagged);
                            is_drawer_open.set(false);
                        },
                        span { "★ Favoritos" }
                    }

                    button {
                        style: if current_filt == InboxFilter::Attachments {
                            "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.6rem; font-size: 0.85rem; font-weight: bold; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center;"
                        } else {
                            "background: #fff; color: #0a0a0a; border: 1px solid transparent; padding: 0.4rem 0.6rem; font-size: 0.85rem; font-weight: bold; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center;"
                        },
                        onclick: move |_| {
                            active_filter.set(InboxFilter::Attachments);
                            is_drawer_open.set(false);
                        },
                        span { "📎 Com Anexos" }
                    }
                }
            }

            // Seção: Pastas e Subpastas Hierárquicas
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 0.8rem; box-shadow: 3px 3px 0 #0a0a0a; display: flex; flex-direction: column; gap: 0.3rem;",
                div {
                    style: "font-size: 0.75rem; font-weight: bold; color: #0a0a0a; text-transform: uppercase; margin-bottom: 0.6rem; letter-spacing: 1px; border-bottom: 1px solid #0a0a0a; padding-bottom: 0.3rem; display: flex; justify-content: space-between;",
                    span { "PASTAS" }
                    span { style: "color: #888; font-size: 0.7rem;", "{mailboxes.read().len()} total" }
                }

                if folder_tree.is_empty() {
                    div {
                        style: "color: #777; font-size: 0.8rem; padding: 0.4rem; font-style: italic;",
                        "Nenhuma pasta carregada."
                    }
                }

                for node in folder_tree.into_iter() {
                    {
                        let is_active = active_box == node.raw_name;
                        let raw = node.raw_name.clone();
                        let padding_left = format!("{}rem", 0.6 + (node.depth as f32 * 0.9));
                        let indent_prefix = if node.depth > 0 { "└── " } else { "" };
                        let icon = node.role.icon();
                        let badge_text = node.badge_text();
                        let has_unread = node.has_unread();

                        rsx! {
                            button {
                                key: "{node.raw_name}",
                                style: if is_active {
                                    format!("background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.6rem 0.4rem {}; font-size: 0.85rem; font-weight: bold; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center; box-shadow: 2px 2px 0 #dc143c; width: 100%; box-sizing: border-box;", padding_left)
                                } else {
                                    format!("background: #fff; color: #0a0a0a; border: 1px solid #e0e0e0; padding: 0.4rem 0.6rem 0.4rem {}; font-size: 0.85rem; font-weight: normal; cursor: pointer; text-align: left; display: flex; justify-content: space-between; align-items: center; width: 100%; box-sizing: border-box;", padding_left)
                                },
                                onclick: move |_| select_mailbox(raw.clone()),

                                span {
                                    style: "white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-right: 0.5rem;",
                                    "{indent_prefix}{icon} {node.display_name}"
                                }

                                span {
                                    style: if has_unread {
                                        "background: #dc143c; color: #fff; padding: 0.1rem 0.4rem; font-weight: bold; font-size: 0.7rem; border-radius: 2px; white-space: nowrap;"
                                    } else if is_active {
                                        "background: #333; color: #ccc; padding: 0.1rem 0.4rem; font-size: 0.7rem; border-radius: 2px; white-space: nowrap;"
                                    } else {
                                        "background: #eee; color: #555; padding: 0.1rem 0.4rem; font-size: 0.7rem; border-radius: 2px; white-space: nowrap;"
                                    },
                                    "{badge_text}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
