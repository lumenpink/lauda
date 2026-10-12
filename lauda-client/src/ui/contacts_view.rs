use dioxus::prelude::*;
use lauda_types::contact::ContactEntry;

use crate::domain::contacts_cache::{
    LocalContactsCache, create_local_contact, format_contact_recipient,
};
use crate::domain::session::SessionConfig;
use crate::ui::state::Screen;

#[component]
pub fn ContactsView(mut screen: Signal<Screen>, session: Signal<SessionConfig>) -> Element {
    let mut contacts = use_signal(Vec::<ContactEntry>::new);
    let mut is_loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);

    let mut show_form = use_signal(|| false);
    let mut input_name = use_signal(String::new);
    let mut input_email = use_signal(String::new);
    let mut input_notes = use_signal(String::new);

    // Carregamento otimista imediato (0ms) a partir do cache local
    use_effect(move || {
        let cache_dir = crate::domain::cache::get_default_cache_dir();
        if let Ok(cache) = LocalContactsCache::open(&cache_dir)
            && let Ok(local_list) = cache.load_contacts()
        {
            contacts.set(local_list);
            is_loading.set(false);
        }
    });

    let handle_save_contact = move |_| {
        let email = input_email.read().trim().to_string();
        let name_val = input_name.read().trim().to_string();
        let notes_val = input_notes.read().trim().to_string();

        if email.is_empty() {
            error_msg.set(Some(
                "E-mail é obrigatório para cadastrar contato.".to_string(),
            ));
            return;
        }

        let sess = session.read().clone();
        let entry = create_local_contact(
            &sess.username,
            if name_val.is_empty() {
                None
            } else {
                Some(&name_val)
            },
            &email,
            if notes_val.is_empty() {
                None
            } else {
                Some(&notes_val)
            },
        );

        let cache_dir = crate::domain::cache::get_default_cache_dir();
        if let Ok(cache) = LocalContactsCache::open(&cache_dir) {
            let token = cache
                .get_sync_token()
                .ok()
                .flatten()
                .unwrap_or_else(|| "ts:0".to_string());
            if let Err(e) = cache.apply_sync_delta(&[entry], &[], &token) {
                error_msg.set(Some(format!("Falha ao salvar no cache local: {e}")));
                return;
            }
            if let Ok(updated) = cache.load_contacts() {
                contacts.set(updated);
            }
        }

        // Limpar formulário e fechar
        input_name.set(String::new());
        input_email.set(String::new());
        input_notes.set(String::new());
        show_form.set(false);
        error_msg.set(None);
    };

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 1rem; max-width: 720px; margin: 0 auto;",

            // Barra de Navegação do Painel de Contatos
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 0.8rem 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; justify-content: space-between; align-items: center;",
                div {
                    style: "display: flex; align-items: center; gap: 0.8rem;",
                    button {
                        style: "background: #fff; color: #0a0a0a; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer;",
                        onclick: move |_| screen.set(Screen::Inbox),
                        "← VOLTAR À CAIXA"
                    }
                    span {
                        style: "background: #0a0a0a; color: #fff; padding: 0.3rem 0.6rem; font-weight: bold; font-size: 0.85rem;",
                        "👥 CONTATOS ({contacts.read().len()})"
                    }
                }
                button {
                    style: "background: #2e8b57; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                    onclick: move |_| {
                        let cur = *show_form.read();
                        show_form.set(!cur);
                    },
                    if *show_form.read() { "✕ FECHAR" } else { "+ NOVO CONTATO" }
                }
            }

            if let Some(err) = error_msg.read().clone() {
                div {
                    style: "border: 2px solid #dc143c; background: #ffebee; color: #b71c1c; padding: 0.6rem; font-weight: bold; font-size: 0.85rem;",
                    "⚠ {err}"
                }
            }

            // Formulário de Criação de Contato
            if *show_form.read() {
                div {
                    style: "border: 2px solid #0a0a0a; background: #fafafa; padding: 1.2rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; flex-direction: column; gap: 0.8rem;",
                    h3 { style: "margin: 0; font-size: 1.1rem; color: #0a0a0a;", "ADICIONAR NOVO CONTATO" }
                    div {
                        label { style: "display: block; font-weight: bold; font-size: 0.8rem; margin-bottom: 0.2rem;", "NOME DE EXIBIÇÃO:" }
                        input {
                            style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 1px solid #0a0a0a; font-family: monospace;",
                            value: "{input_name}",
                            oninput: move |evt| input_name.set(evt.value()),
                            placeholder: "Ex: Carolina Maria de Jesus",
                        }
                    }
                    div {
                        label { style: "display: block; font-weight: bold; font-size: 0.8rem; margin-bottom: 0.2rem;", "E-MAIL (OBRIGATÓRIO):" }
                        input {
                            style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 2px solid #0a0a0a; font-family: monospace;",
                            value: "{input_email}",
                            oninput: move |evt| input_email.set(evt.value()),
                            placeholder: "Ex: carolina@quarto.org",
                        }
                    }
                    div {
                        label { style: "display: block; font-weight: bold; font-size: 0.8rem; margin-bottom: 0.2rem;", "NOTAS / OBSERVAÇÕES:" }
                        input {
                            style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 1px solid #0a0a0a; font-family: monospace;",
                            value: "{input_notes}",
                            oninput: move |evt| input_notes.set(evt.value()),
                            placeholder: "Notas opcionais...",
                        }
                    }
                    button {
                        style: "background: #2e8b57; color: #fff; border: 2px solid #0a0a0a; padding: 0.5rem 1rem; font-weight: bold; cursor: pointer; align-self: flex-start; box-shadow: 2px 2px 0 #0a0a0a;",
                        onclick: handle_save_contact,
                        "SALVAR CONTATO ✔"
                    }
                }
            }

            if *is_loading.read() {
                div {
                    style: "border: 2px dashed #0a0a0a; background: #fff; padding: 2rem; text-align: center; color: #555;",
                    "Carregando catálogo de contatos..."
                }
            } else if contacts.read().is_empty() {
                div {
                    style: "border: 2px dashed #0a0a0a; background: #fff; padding: 2rem; text-align: center; color: #555;",
                    "Nenhum contato registrado no cache local. Use o botão acima para adicionar."
                }
            } else {
                div {
                    style: "display: flex; flex-direction: column; gap: 0.6rem;",
                    for c in contacts.read().iter() {
                        {
                            let id = c.id.clone();
                            let email = c.primary_email.clone();
                            let name_opt = c.display_name.clone();
                            let has_auto = c.has_autocrypt;
                            let fp_opt = c.autocrypt_fingerprint.clone();
                            let formatted_recipient = format_contact_recipient(name_opt.as_deref(), &email);

                            rsx! {
                                div {
                                    key: "{id}",
                                    style: "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 0.8rem;",
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 0.2rem;",
                                        div {
                                            style: "display: flex; align-items: center; gap: 0.5rem;",
                                            span { style: "font-weight: bold; font-size: 1rem; color: #0a0a0a;", "{email}" }
                                            if let Some(name) = name_opt {
                                                span { style: "font-size: 0.85rem; color: #555;", "({name})" }
                                            }
                                        }
                                        div {
                                            style: "display: flex; align-items: center; gap: 0.6rem; margin-top: 0.2rem;",
                                            if has_auto {
                                                span {
                                                    style: "background: #2e8b57; color: #fff; font-size: 0.75rem; font-weight: bold; padding: 0.15rem 0.4rem;",
                                                    "🔒 PGP / AUTOCRYPT ATIVO"
                                                }
                                            } else {
                                                span {
                                                    style: "background: #777; color: #fff; font-size: 0.75rem; font-weight: bold; padding: 0.15rem 0.4rem;",
                                                    "🔓 TEXTO PLANO"
                                                }
                                            }
                                            if let Some(fp) = fp_opt {
                                                span { style: "font-family: monospace; font-size: 0.7rem; color: #444;", "FP: {fp}" }
                                            }
                                        }
                                    }
                                    div {
                                        style: "display: flex; gap: 0.5rem; align-items: center;",
                                        button {
                                            style: "background: #dc143c; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                                            onclick: move |_| screen.set(Screen::Compose {
                                                reply_to: Some(formatted_recipient.clone()),
                                                reply_cc: None,
                                                reply_subject: None,
                                                initial_body: None,
                                            }),
                                            "ESCREVER ✉"
                                        }
                                        button {
                                            style: "background: #fff; color: #b71c1c; border: 1px solid #b71c1c; padding: 0.4rem 0.6rem; font-weight: bold; cursor: pointer;",
                                            onclick: {
                                                let del_id = id.clone();
                                                move |_| {
                                                    let cache_dir = crate::domain::cache::get_default_cache_dir();
                                                    if let Ok(cache) = LocalContactsCache::open(&cache_dir) {
                                                        let token = cache.get_sync_token().ok().flatten().unwrap_or_else(|| "ts:0".to_string());
                                                        let _ = cache.apply_sync_delta(&[], std::slice::from_ref(&del_id), &token);
                                                        if let Ok(updated) = cache.load_contacts() {
                                                            contacts.set(updated);
                                                        }
                                                    }
                                                }
                                            },
                                            "🗑"
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
