use dioxus::prelude::*;
use lauda_types::{MailboxInfo, Message, MessageEnvelope, message::SignatureStatus};

use crate::adapters::HttpApiClient;
use crate::domain::cache::{LocalMessageCache, get_default_cache_dir};
use crate::domain::ports::MailApiClient;
use crate::domain::session::{SessionConfig, clear_session};
use crate::domain::sync::{ActionType, ActionsQueue};
use crate::domain::time::current_unix_timestamp;
use crate::ui::state::Screen;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InboxFilter {
    #[default]
    All,
    Unread,
    Flagged,
    Attachments,
}

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
    let mut is_loading_more = use_signal(|| false);
    let mut has_more = use_signal(|| true);
    let mut search_query = use_signal(String::new);
    let mut active_filter = use_signal(|| InboxFilter::All);
    let mut is_drawer_open = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let mut loading_uid = use_signal(|| None::<u32>);

    use_effect(move || {
        let _ = current_mailbox.read();
        has_more.set(true);
    });

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
                if let Ok(msgs) = client
                    .list_messages(&token, &box_name, Some(50), Some(0))
                    .await
                {
                    has_more.set(msgs.len() == 50);
                    messages.set(msgs);
                }
                // Sincroniza a fatia de 90 dias em background no cache local
                if let Ok(docs) = client.fetch_index_slice(&token, &box_name, 90).await {
                    let cache = LocalMessageCache::new(get_default_cache_dir());
                    let _ = cache.save_index_slice(&box_name, &docs);
                }
                is_refreshing.set(false);
            });
        }
    };

    let handle_load_more = move |_| {
        let sess = session.read().clone();
        if let Some(token) = sess.token.clone() {
            let box_name = current_mailbox.read().clone();
            let current_count = messages.read().len();
            is_loading_more.set(true);

            spawn(async move {
                let client = HttpApiClient::new(sess.server_url);
                if let Ok(more_msgs) = client
                    .list_messages(&token, &box_name, Some(50), Some(current_count))
                    .await
                {
                    if more_msgs.len() < 50 {
                        has_more.set(false);
                    }
                    messages.write().extend(more_msgs);
                }
                is_loading_more.set(false);
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

    let search_resource = use_resource(move || {
        let q = search_query.read().trim().to_string();
        let box_name = current_mailbox.read().clone();
        let sess = session.read().clone();
        async move {
            if !crate::domain::is_search_active(&q) {
                None
            } else if let Some(token) = sess.token {
                let client = HttpApiClient::new(sess.server_url);
                match client.search_messages(&token, &box_name, &q).await {
                    Ok(result) => Some((result, false)),
                    Err(_) => {
                        let cache = LocalMessageCache::new(get_default_cache_dir());
                        if let Ok(Some(local_index)) = cache.load_index_slice(&box_name) {
                            let provider = lauda_index::SimpleCpuEmbeddingProvider;
                            let ranked = local_index.search_hybrid(&q, &provider, 0.5);
                            let docs = local_index.documents();
                            let envelopes: Vec<MessageEnvelope> = ranked
                                .into_iter()
                                .filter_map(|r| {
                                    docs.get(&r.uid).map(|doc| MessageEnvelope {
                                        uid: doc.uid,
                                        subject: doc.subject.clone(),
                                        sender: doc.sender.clone(),
                                        date: doc.date.clone(),
                                        cc: None,
                                        has_attachments: doc.has_attachments,
                                        is_read: doc.is_read,
                                        is_flagged: doc.is_flagged,
                                        crypto_meta: None,
                                    })
                                })
                                .collect();
                            let total = envelopes.len() as u64;
                            let fallback = lauda_types::SearchResult {
                                messages: envelopes,
                                sync_status: lauda_types::MailboxSyncStatus::new(total, total),
                            };
                            Some((fallback, true))
                        } else {
                            None
                        }
                    }
                }
            } else {
                None
            }
        }
    });

    let current_filt = *active_filter.read();
    let query_str = search_query.read().trim().to_string();
    let is_searching = crate::domain::is_search_active(&query_str);

    let (base_list, is_offline_search, search_sync_status) = match &*search_resource.read() {
        Some(Some((result, is_offline))) => (
            result.messages.clone(),
            *is_offline,
            Some(result.sync_status.clone()),
        ),
        _ => (
            crate::domain::fast_filter_envelopes(&messages.read(), &query_str),
            false,
            None,
        ),
    };

    let filtered_messages: Vec<MessageEnvelope> = if !is_searching {
        messages
            .read()
            .iter()
            .filter(|env| match current_filt {
                InboxFilter::All => true,
                InboxFilter::Unread => !env.is_read,
                InboxFilter::Flagged => env.is_flagged,
                InboxFilter::Attachments => env.has_attachments,
            })
            .cloned()
            .collect()
    } else {
        base_list
            .into_iter()
            .filter(|env| match current_filt {
                InboxFilter::All => true,
                InboxFilter::Unread => !env.is_read,
                InboxFilter::Flagged => env.is_flagged,
                InboxFilter::Attachments => env.has_attachments,
            })
            .collect()
    };

    rsx! {
            div {
                style: "display: flex; flex-direction: column; gap: 1rem;",

                if let Some(ref err) = *error_msg.read() {
                    div {
                        style: "border: 2px solid #dc143c; background: #ffebee; color: #b71c1c; padding: 0.8rem 1rem; font-weight: bold; font-size: 0.9rem; display: flex; justify-content: space-between; align-items: center;",
                        span { "⚠ {err}" }
                        button {
                            style: "background: none; border: none; font-weight: bold; cursor: pointer; color: #b71c1c;",
                            onclick: move |_| error_msg.set(None),
                            "✕"
                        }
                    }
                }

                // Barra de Controle e Ações
                div {
                    style: "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 0.8rem;",
                    div {
                        style: "display: flex; align-items: center; gap: 0.8rem; flex-wrap: wrap;",
                        span {
                            style: "background: #0a0a0a; color: #fff; padding: 0.3rem 0.6rem; font-weight: bold; font-size: 0.85rem;",
                            "CONTA: {username}"
                        }
                        button {
                            style: "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c; display: flex; align-items: center; gap: 0.4rem;",
                            onclick: move |_| {
                                let curr = *is_drawer_open.read();
                                is_drawer_open.set(!curr);
                            },
                            span { "☰ PASTAS" }
                            span {
                                style: "background: #dc143c; color: #fff; padding: 0.1rem 0.4rem; font-size: 0.75rem; border-radius: 2px;",
                                "{current_mailbox.read()}"
                            }
                        }
                    }
                    div {
                        style: "display: flex; gap: 0.5rem;",
                        button {
                            style: "background: #dc143c; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                            onclick: move |_| screen.set(Screen::Compose {
                                reply_to: None,
                                reply_cc: None,
                                reply_subject: None,
                                initial_body: None,
                            }),
                            "+ NOVA MENSAGEM"
                        }
                        button {
                            style: "background: #fff; color: #0a0a0a; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                            onclick: move |_| screen.set(Screen::Calendar),
                            "📅 AGENDA"
                        }
                        button {
                            style: "background: #fff; color: #0a0a0a; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                            onclick: move |_| screen.set(Screen::Contacts),
                            "👥 CONTATOS"
                        }
                        button {
                            style: "background: #fff; color: #0a0a0a; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                            onclick: move |_| screen.set(Screen::Rules { initial_rule: None }),
                            "⚙ REGRAS"
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

                // Modal / Drawer Retrátil de Navegação (Mobile-friendly com backdrop)
                if *is_drawer_open.read() {
                    div {
                        style: "position: fixed; inset: 0; background: rgba(10, 10, 10, 0.7); z-index: 999; display: flex; justify-content: flex-start; align-items: stretch;",
                        onclick: move |_| is_drawer_open.set(false),

                        div {
                            style: "background: #f4efe6; width: 320px; max-width: 85vw; height: 100%; border-right: 3px solid #0a0a0a; box-shadow: 5px 0 0 #dc143c; padding: 1.2rem; box-sizing: border-box; overflow-y: auto; display: flex; flex-direction: column; gap: 1rem;",
                            onclick: move |e| e.stop_propagation(),

                            div {
                                style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 2px solid #0a0a0a; padding-bottom: 0.6rem;",
                                span { style: "font-weight: bold; font-size: 1.1rem; color: #0a0a0a;", "NAVEGAÇÃO" }
                                button {
                                    style: "background: #0a0a0a; color: #fff; border: none; padding: 0.3rem 0.6rem; font-weight: bold; cursor: pointer;",
                                    onclick: move |_| is_drawer_open.set(false),
                                    "✕ FECHAR"
                                }
                            }

                            crate::ui::mailbox_nav::MailboxNavigation {
                                session,
                                mailboxes,
                                current_mailbox,
                                messages,
                                active_filter,
                                is_drawer_open,
                            }
                        }
                    }
                }

                // Área Principal com Suporte a Visualizador Colunar / Sidebar no Desktop
                div {
                    style: "display: flex; gap: 1.2rem; align-items: flex-start;",

                    // Sidebar fixada visível em telas largas (Desktop)
                    div {
                        class: "desktop-mailbox-sidebar",
                        style: "width: 280px; min-width: 250px; display: none; flex-direction: column; gap: 1rem;",
                        crate::ui::mailbox_nav::MailboxNavigation {
                            session,
                            mailboxes,
                            current_mailbox,
                            messages,
                            active_filter,
                            is_drawer_open,
                        }
                    }

                    // Coluna Central de Mensagens
                    div {
                        style: "flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.8rem;",

                        // Barra de Busca Rápida
                        div {
                            style: "border: 2px solid #0a0a0a; background: #fff; padding: 0.6rem 0.8rem; box-shadow: 3px 3px 0 #0a0a0a; display: flex; align-items: center; gap: 0.5rem;",
                            span { style: "color: #dc143c; font-weight: bold;", "🔍" }
                            input {
                                style: "border: none; outline: none; width: 100%; font-family: monospace; font-size: 0.95rem; background: transparent; color: #0a0a0a;",
                                placeholder: "Pesquisar por assunto ou remetente (mín. 3 caracteres)...",
                                value: "{search_query.read()}",
                                oninput: move |e| search_query.set(e.value()),
                            }
                            if !search_query.read().is_empty() {
                                button {
                                    style: "background: none; border: none; cursor: pointer; color: #888; font-weight: bold; font-family: monospace;",
                                    onclick: move |_| search_query.set(String::new()),
                                    "✕"
                                }
                            }
                        }

                        if let Some(hint_msg) = crate::domain::format_search_hint(&search_query.read()) {
                            div {
                                style: "border: 1px solid #0a0a0a; background: #fff5f5; color: #dc143c; padding: 0.3rem 0.6rem; font-family: monospace; font-size: 0.75rem; font-weight: bold; display: flex; align-items: center; gap: 0.4rem; box-shadow: 2px 2px 0 #0a0a0a;",
                                span { "ℹ" }
                                span { "{hint_msg}" }
                            }
                        }

                        if is_offline_search && is_searching {
                            div {
                                style: "border: 2px solid #dc143c; background: #0a0a0a; color: #fff; padding: 0.5rem 0.8rem; font-family: monospace; font-size: 0.8rem; font-weight: bold; display: flex; align-items: center; gap: 0.5rem; box-shadow: 2px 2px 0 #dc143c;",
                                span { "⚡" }
                                span { "MODO OFFLINE: PESQUISANDO NO HISTÓRICO LOCAL DOS ÚLTIMOS 90 DIAS" }
                            }
                        }

                        if is_searching {
                            if let Some(sync) = &search_sync_status {
                                if !sync.is_synced && !is_offline_search {
                                    div {
                                        style: "border: 2px solid #dc143c; background: #fff5f5; color: #0a0a0a; padding: 0.6rem 0.8rem; font-family: monospace; font-size: 0.8rem; box-shadow: 3px 3px 0 #dc143c; display: flex; flex-direction: column; gap: 0.3rem;",
                                        div {
                                            style: "display: flex; align-items: center; gap: 0.4rem; font-weight: bold; color: #dc143c;",
                                            span { "⚠️" }
                                            span { "ÍNDICE EM SINCRONIZAÇÃO: {sync.indexed_count} de {sync.total_count} e-mails indexados (faltam {sync.remaining_count})" }
                                        }
                                        div {
                                            style: "font-size: 0.75rem; color: #444;",
                                            "A busca cobriu apenas o acervo já indexado. É possível que o e-mail procurado ainda não tenha entrado no índice."
                                        }
                                    }
                                }
                            }
                        }

                        // Barra de Filtros Rápidos (Pills)
                        div {
                            style: "display: flex; gap: 0.4rem; flex-wrap: wrap; align-items: center;",
                            button {
                                style: if *active_filter.read() == InboxFilter::All {
                                    "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.3rem 0.6rem; font-size: 0.75rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c;"
                                } else {
                                    "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.3rem 0.6rem; font-size: 0.75rem; font-weight: bold; cursor: pointer;"
                                },
                                onclick: move |_| active_filter.set(InboxFilter::All),
                                "TODOS"
                            }
                            button {
                                style: if *active_filter.read() == InboxFilter::Unread {
                                    "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.3rem 0.6rem; font-size: 0.75rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c;"
                                } else {
                                    "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.3rem 0.6rem; font-size: 0.75rem; font-weight: bold; cursor: pointer;"
                                },
                                onclick: move |_| active_filter.set(InboxFilter::Unread),
                                "NÃO LIDOS"
                            }
                            button {
                                style: if *active_filter.read() == InboxFilter::Flagged {
                                    "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.3rem 0.6rem; font-size: 0.75rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c;"
                                } else {
                                    "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.3rem 0.6rem; font-size: 0.75rem; font-weight: bold; cursor: pointer;"
                                },
                                onclick: move |_| active_filter.set(InboxFilter::Flagged),
                                "★ FAVORITOS"
                            }
                            button {
                                style: if *active_filter.read() == InboxFilter::Attachments {
                                    "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.3rem 0.6rem; font-size: 0.75rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c;"
                                } else {
                                    "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.3rem 0.6rem; font-size: 0.75rem; font-weight: bold; cursor: pointer;"
                                },
                                onclick: move |_| active_filter.set(InboxFilter::Attachments),
                                "📎 COM ANEXOS"
                            }
                        }

                // Lista de Mensagens
                div {
                    style: "display: flex; flex-direction: column; gap: 0.6rem;",
                    if filtered_messages.is_empty() {
                        div {
                            style: "border: 2px dashed #666; background: #fff; padding: 2rem; text-align: center; color: #555;",
                            if let Some(hint_msg) = crate::domain::format_search_hint(&query_str) {
                                "{hint_msg}"
                            } else if is_searching {
                                "Nenhuma mensagem encontrada para \"{query_str}\" nos filtros ativos."
                            } else if messages.read().is_empty() {
                                "⌛ SINCRONIZANDO OU NENHUMA MENSAGEM NA CAIXA..."
                            } else {
                                "Nenhuma mensagem corresponde aos filtros ativos."
                            }
                        }
                    }
                    for env in filtered_messages.iter() {
                        {
                            let uid = env.uid;
                            let unread = !env.is_read;
                            let is_flagged = env.is_flagged;
                            let has_att = env.has_attachments;
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
                                        let cache_dir = get_default_cache_dir();
                                        let cache = LocalMessageCache::new(&cache_dir);
                                        let actions = ActionsQueue::new(&cache_dir);
                                        let now_unix = current_unix_timestamp();

                                        let was_unread = unread;
                                        if was_unread {
                                            // Atualização reativa imediata na UI local (optimistic update)
                                            if let Some(m) = messages.write().iter_mut().find(|m| m.uid == uid) {
                                                m.is_read = true;
                                            }
                                            if let Some(mb) = mailboxes.write().iter_mut().find(|mb| mb.name == box_name)
                                                && mb.unread_messages > 0
                                            {
                                                mb.unread_messages -= 1;
                                            }
                                            // Persiste imediatamente no cache local (SQLite / Storage)
                                            let _ = cache.mark_envelope_read(&box_name, uid, true);
                                            // Enfileira na fila offline para garantir resiliência
                                            let _ = actions.enqueue(uid, &box_name, ActionType::MarkRead { read: true }, now_unix);
                                        }

                                        // 1. Abertura atômica imediata via cache local (0ms de latência)
                                        if let Ok(Some(mut cached_msg)) = cache.load_message(&box_name, uid) {
                                            if was_unread {
                                                cached_msg.envelope.is_read = true;
                                            }
                                            active_message.set(Some(cached_msg));
                                            screen.set(Screen::MessageDetail(uid));
                                            return;
                                        }

                                        // 2. Se não estiver no cache local, busca na rede
                                        if let Some(token) = sess.token.clone() {
                                            loading_uid.set(Some(uid));
                                            error_msg.set(None);

                                            spawn(async move {
                                                let client = HttpApiClient::new(sess.server_url.clone());
                                                match client.get_message(&token, &box_name, uid).await {
                                                    Ok(mut msg) => {
                                                        if was_unread {
                                                            msg.envelope.is_read = true;
                                                        }
                                                        let cache = LocalMessageCache::new(get_default_cache_dir());
                                                        let _ = cache.save_message(&box_name, &msg);

                                                        active_message.set(Some(msg));
                                                        screen.set(Screen::MessageDetail(uid));
                                                    }
                                                    Err(e) => {
                                                        // Reverter estado se falhou ao abrir a mensagem
                                                        if was_unread {
                                                            if let Some(m) = messages.write().iter_mut().find(|m| m.uid == uid) {
                                                                m.is_read = false;
                                                            }
                                                            if let Some(mb) = mailboxes.write().iter_mut().find(|mb| mb.name == box_name) {
                                                                mb.unread_messages += 1;
                                                            }
                                                            let _ = cache.mark_envelope_read(&box_name, uid, false);
                                                        }
                                                        error_msg.set(Some(format!("Falha ao abrir mensagem #{}: {}", uid, e)));
                                                    }
                                                }
                                                loading_uid.set(None);
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
                                        div {
                                            style: "display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap;",
                                            if let Some(ref crypto) = env.crypto_meta {
                                                if crypto.is_encrypted {
                                                    span {
                                                        style: "background: #0a0a0a; color: #fff; font-size: 0.75rem; font-weight: bold; padding: 0.1rem 0.35rem; border: 1px solid #0a0a0a; letter-spacing: 0.05em;",
                                                        "[🔒]"
                                                    }
                                                }
                                                if crypto.signature_status == SignatureStatus::Valid {
                                                    span {
                                                        style: "background: #d1fae5; color: #047857; font-size: 0.75rem; font-weight: bold; padding: 0.1rem 0.35rem; border: 1px solid #059669; letter-spacing: 0.05em;",
                                                        "[✓ PGP]"
                                                    }
                                                } else if crypto.signature_status == SignatureStatus::Invalid {
                                                    span {
                                                        style: "background: #fee2e2; color: #dc2626; font-size: 0.75rem; font-weight: bold; padding: 0.1rem 0.35rem; border: 1px solid #dc2626; letter-spacing: 0.05em;",
                                                        "[⚠ INVÁLIDA]"
                                                    }
                                                }
                                            }
                                            if has_att {
                                                span { style: "font-size: 0.85rem; color: #444;", "📎" }
                                            }
                                            if is_flagged {
                                                span { style: "color: #dc143c; font-size: 0.95rem; font-weight: bold;", "★" }
                                            }
                                            if *loading_uid.read() == Some(uid) {
                                                span { style: "color: #dc143c; font-size: 0.8rem; font-weight: bold; animation: pulse 1s infinite;", "CARREGANDO..." }
                                            }
                                            span { style: "font-size: 0.8rem; color: #666; margin-left: 0.3rem;", "{date}" }
                                        }
                                    }
                                    div {
                                        style: "font-size: 1.05rem; font-weight: {font_weight}; color: #0a0a0a; margin-top: 0.2rem;",
                                        "{subject}"
                                    }
                                }
                            }
                        }
                    }
                    if !is_searching && *has_more.read() && !filtered_messages.is_empty() {
                        button {
                            style: "width: 100%; padding: 0.8rem; margin-top: 1rem; border: 2px solid #0a0a0a; background: #faf9f6; color: #0a0a0a; font-family: monospace; font-weight: bold; cursor: pointer; box-shadow: 3px 3px 0 #dc143c; text-align: center; text-transform: uppercase;",
                            disabled: *is_loading_more.read(),
                            onclick: handle_load_more,
                            if *is_loading_more.read() {
                                "CARREGANDO MAIS MENSAGENS..."
                            } else {
                                "[ + CARREGAR MAIS 50 MENSAGENS ]"
                            }
                        }
                    }
                }
            }
        }
    }
    }
}
