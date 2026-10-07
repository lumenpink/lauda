use dioxus::prelude::*;
use lauda_types::ContactInfo;

use crate::adapters::HttpApiClient;
use crate::domain::ports::MailApiClient;
use crate::domain::session::SessionConfig;
use crate::ui::state::Screen;

#[component]
pub fn ContactsView(mut screen: Signal<Screen>, session: Signal<SessionConfig>) -> Element {
    let mut contacts = use_signal(Vec::<ContactInfo>::new);
    let mut is_loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);

    use_effect(move || {
        let sess = session.read().clone();
        if let Some(token) = sess.token {
            spawn(async move {
                let client = HttpApiClient::new(sess.server_url);
                match client.list_contacts(&token).await {
                    Ok(list) => {
                        contacts.set(list);
                        is_loading.set(false);
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Falha ao carregar contatos: {}", e)));
                        is_loading.set(false);
                    }
                }
            });
        }
    });

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 1rem;",

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
                        "👥 CONTATOS & AUTOCRYPT"
                    }
                }
            }

            if let Some(err) = error_msg.read().clone() {
                div {
                    style: "background: #8b0000; color: #fff; padding: 0.6rem 1rem; font-weight: bold; font-size: 0.85rem;",
                    "{err}"
                }
            }

            if *is_loading.read() {
                div {
                    style: "border: 2px dashed #0a0a0a; background: #fff; padding: 2rem; text-align: center; color: #555;",
                    "Carregando catálogo de contatos e chaves..."
                }
            } else if contacts.read().is_empty() {
                div {
                    style: "border: 2px dashed #0a0a0a; background: #fff; padding: 2rem; text-align: center; color: #555;",
                    "Nenhum contato registrado até o momento."
                }
            } else {
                div {
                    style: "display: flex; flex-direction: column; gap: 0.6rem;",
                    for c in contacts.read().iter() {
                        {
                            let email = c.email.clone();
                            let name_opt = c.name.clone();
                            let count = c.message_count;
                            let has_auto = c.has_autocrypt;
                            let fp_opt = c.autocrypt_fingerprint.clone();
                            let compose_email = email.clone();

                            rsx! {
                                div {
                                    key: "{email}",
                                    style: "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 0.8rem;",
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 0.2rem;",
                                        div {
                                            style: "display: flex; align-items: center; gap: 0.5rem;",
                                            span { style: "font-weight: bold; font-size: 1rem; color: #0a0a0a;", "{email}" }
                                            if let Some(name) = name_opt {
                                                span { style: "font-size: 0.85rem; color: #666;", "({name})" }
                                            }
                                        }
                                        div {
                                            style: "display: flex; align-items: center; gap: 0.6rem; margin-top: 0.2rem;",
                                            span { style: "font-size: 0.8rem; color: #555;", "{count} mensagens" }
                                            if has_auto {
                                                span {
                                                    style: "background: #2e8b57; color: #fff; font-size: 0.75rem; font-weight: bold; padding: 0.15rem 0.4rem;",
                                                    "🔒 AUTOCRYPT ATIVO"
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
                                    button {
                                        style: "background: #dc143c; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                                        onclick: move |_| screen.set(Screen::Compose {
                                            reply_to: Some(compose_email.clone()),
                                            reply_cc: None,
                                            reply_subject: None,
                                            initial_body: None,
                                        }),
                                        "ESCREVER ✉"
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
