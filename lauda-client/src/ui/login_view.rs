use dioxus::prelude::*;
use lauda_types::{LoginRequest, MailboxInfo, MessageEnvelope};

use crate::adapters::HttpApiClient;
use crate::domain::ports::MailApiClient;
use crate::domain::session::{SessionConfig, save_session};
use crate::ui::state::Screen;

#[component]
pub fn LoginView(
    screen: Signal<Screen>,
    session: Signal<SessionConfig>,
    mailboxes: Signal<Vec<MailboxInfo>>,
    current_mailbox: Signal<String>,
    messages: Signal<Vec<MessageEnvelope>>,
) -> Element {
    let initial_session = session.read().clone();
    let mut server_input = use_signal(|| initial_session.server_url);
    let mut username_input = use_signal(|| initial_session.username);
    let mut password_input = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let mut show_server_config = use_signal(|| false);

    let handle_login = move |_| {
        let server = server_input.read().trim().to_string();
        let user = username_input.read().trim().to_string();
        let pass = password_input.read().trim().to_string();

        if server.is_empty() {
            error_msg.set(Some("Preencha o campo: SERVIDOR LAUDA (URL)".to_string()));
            return;
        }
        if user.is_empty() {
            error_msg.set(Some("Preencha o campo: USUÁRIO / CONTA".to_string()));
            return;
        }
        if pass.is_empty() {
            error_msg.set(Some("Preencha o campo: SENHA".to_string()));
            return;
        }

        is_loading.set(true);
        error_msg.set(None);

        spawn(async move {
            let client = HttpApiClient::new(server.clone());
            let req = LoginRequest {
                username: user.clone(),
                password: pass,
            };

            match client.login(&req).await {
                Ok(token_resp) => {
                    let post_login = crate::domain::session::prepare_post_login_state(
                        server,
                        user,
                        token_resp.token,
                    );

                    let _ = save_session(&post_login.session, None);
                    current_mailbox.set(post_login.initial_mailbox);
                    session.set(post_login.session);

                    is_loading.set(false);
                    screen.set(post_login.target_screen);
                }
                Err(err) => {
                    is_loading.set(false);
                    error_msg.set(Some(format!("Falha na autenticação: {}", err)));
                }
            }
        });
    };

    rsx! {
        div {
            style: "max-width: 480px; margin: 2rem auto; border: 2px solid #0a0a0a; background: #fff; padding: 2rem; box-shadow: 6px 6px 0 #0a0a0a;",
            div {
                style: "margin-bottom: 1.5rem;",
                span {
                    style: "background: #dc143c; color: #fff; padding: 0.2rem 0.5rem; font-weight: bold; font-size: 0.8rem;",
                    "AUTENTICAÇÃO SEGURA"
                }
                h2 {
                    style: "font-size: 1.5rem; margin: 0.5rem 0 0 0; color: #0a0a0a;",
                    "CONECTAR AO SERVIDOR"
                }
                p {
                    style: "color: #555; font-size: 0.9rem; margin-top: 0.3rem;",
                    "Credenciais serão gravadas localmente em arquivo de sessão protegido."
                }
            }

            if let Some(err) = error_msg.read().as_ref() {
                div {
                    style: "border: 2px solid #dc143c; background: #ffebee; color: #b71c1c; padding: 0.8rem; margin-bottom: 1rem; font-weight: bold; font-size: 0.9rem;",
                    "⚠ {err}"
                }
            }

            div {
                style: "display: flex; flex-direction: column; gap: 1rem;",
                // Servidor Conectado: Selo Trash Polka compacto com botão de expansão discreto
                div {
                    style: "border: 1px solid #0a0a0a; background: #faf9f6; padding: 0.5rem 0.8rem; display: flex; justify-content: space-between; align-items: center; gap: 0.5rem;",
                    div {
                        style: "display: flex; align-items: center; gap: 0.4rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                        span { style: "color: #dc143c; font-weight: bold; font-size: 0.75rem;", "SERVIDOR:" }
                        span { style: "font-family: monospace; font-size: 0.85rem; font-weight: bold; color: #0a0a0a; overflow: hidden; text-overflow: ellipsis;", "{server_input}" }
                    }
                    button {
                        style: "background: none; border: none; cursor: pointer; color: #555; font-size: 0.75rem; text-decoration: underline; font-family: monospace; white-space: nowrap; padding: 0.2rem;",
                        r#type: "button",
                        onclick: move |_| {
                            let curr = *show_server_config.read();
                            show_server_config.set(!curr);
                        },
                        if *show_server_config.read() { "OCULTAR" } else { "ALTERAR" }
                    }
                }

                if *show_server_config.read() {
                    div {
                        style: "border-left: 3px solid #dc143c; padding-left: 0.8rem; margin-top: -0.3rem;",
                        label {
                            r#for: "server_url",
                            style: "display: block; font-weight: bold; font-size: 0.8rem; margin-bottom: 0.3rem; color: #555;",
                            "URL DO SERVIDOR:"
                        }
                        input {
                            id: "server_url",
                            name: "server_url",
                            autocomplete: "url",
                            style: "width: 100%; box-sizing: border-box; padding: 0.5rem; border: 2px solid #0a0a0a; font-family: monospace; font-size: 0.9rem;",
                            value: "{server_input}",
                            oninput: move |evt| server_input.set(evt.value()),
                            onchange: move |evt| server_input.set(evt.value()),
                            placeholder: "http://localhost:8432",
                        }
                    }
                }

                div {
                    label {
                        r#for: "username",
                        style: "display: block; font-weight: bold; font-size: 0.85rem; margin-bottom: 0.3rem;",
                        "USUÁRIO / CONTA:"
                    }
                    input {
                        id: "username",
                        name: "username",
                        autocomplete: "username",
                        style: "width: 100%; box-sizing: border-box; padding: 0.6rem; border: 2px solid #0a0a0a; font-family: monospace; font-size: 1rem;",
                        value: "{username_input}",
                        oninput: move |evt| username_input.set(evt.value()),
                        onchange: move |evt| username_input.set(evt.value()),
                        placeholder: "lumen",
                    }
                }

                div {
                    label {
                        r#for: "password",
                        style: "display: block; font-weight: bold; font-size: 0.85rem; margin-bottom: 0.3rem;",
                        "SENHA:"
                    }
                    input {
                        id: "password",
                        name: "password",
                        autocomplete: "current-password",
                        style: "width: 100%; box-sizing: border-box; padding: 0.6rem; border: 2px solid #0a0a0a; font-family: monospace; font-size: 1rem;",
                        r#type: "password",
                        value: "{password_input}",
                        oninput: move |evt| password_input.set(evt.value()),
                        onchange: move |evt| password_input.set(evt.value()),
                        placeholder: "••••••••",
                    }
                }

                button {
                    style: if *is_loading.read() { "width: 100%; padding: 0.8rem; background: #666; color: #fff; font-weight: bold; border: 2px solid #0a0a0a; cursor: not-allowed; margin-top: 0.5rem;" } else { "width: 100%; padding: 0.8rem; background: #0a0a0a; color: #fff; font-weight: bold; border: 2px solid #0a0a0a; cursor: pointer; margin-top: 0.5rem; box-shadow: 4px 4px 0 #dc143c;" },
                    disabled: *is_loading.read(),
                    onclick: handle_login,
                    if *is_loading.read() {
                        "CONECTANDO..."
                    } else {
                        "ENTRAR / CONECTAR"
                    }
                }

                div {
                    style: "text-align: center; margin-top: 1.2rem; font-size: 0.72rem; color: #888; letter-spacing: 1px;",
                    "LAUDA // v{env!(\"LAUDA_APP_VERSION\")} (dx0.7)"
                }
            }
        }
    }
}
