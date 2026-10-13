use dioxus::prelude::*;
use lauda_types::sieve::{
    ConditionField, MatchType, SieveAction, SieveCondition, SieveRule, SieveScriptBundle,
    generate_sieve_script, parse_sieve_script,
};

use crate::adapters::HttpApiClient;
use crate::domain::ports::MailApiClient;
use crate::domain::session::SessionConfig;
use crate::ui::state::Screen;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    #[default]
    Visual,
    Hacker,
}

#[component]
pub fn RulesView(
    mut screen: Signal<Screen>,
    session: Signal<SessionConfig>,
    initial_rule: Option<SieveRule>,
) -> Element {
    let mut editor_mode = use_signal(|| EditorMode::Visual);
    let mut raw_script = use_signal(String::new);
    let mut rules = use_signal(Vec::<SieveRule>::new);
    let mut is_loading = use_signal(|| false);
    let mut status_msg = use_signal(|| None::<(String, bool)>); // (msg, is_error)

    // Inicializar carregando regras do servidor
    let init_rule_clone = initial_rule.clone();
    use_effect(move || {
        let sess = session.read().clone();
        if let Some(token) = sess.token.clone() {
            let init_rule = init_rule_clone.clone();
            is_loading.set(true);
            spawn(async move {
                let client = HttpApiClient::new(sess.server_url);
                if let Ok(bundle) = client.get_rules(&token).await {
                    let mut loaded_rules = bundle.rules;
                    if let Some(rule) = init_rule {
                        // Inserir a regra preditiva se fornecida
                        if !loaded_rules.iter().any(|r| r.name == rule.name) {
                            loaded_rules.insert(0, rule);
                        }
                    }
                    let generated = generate_sieve_script(&loaded_rules);
                    raw_script.set(if bundle.raw_script.trim().is_empty() {
                        generated
                    } else {
                        bundle.raw_script
                    });
                    rules.set(loaded_rules);
                }
                is_loading.set(false);
            });
        }
    });

    let handle_save = move |_| {
        let sess = session.read().clone();
        if let Some(token) = sess.token.clone() {
            let script_to_save = match *editor_mode.read() {
                EditorMode::Visual => generate_sieve_script(&rules.read()),
                EditorMode::Hacker => raw_script.read().clone(),
            };

            let parsed = parse_sieve_script(&script_to_save);
            let bundle = SieveScriptBundle {
                raw_script: script_to_save.clone(),
                rules: parsed.clone(),
            };

            is_loading.set(true);
            spawn(async move {
                let client = HttpApiClient::new(sess.server_url);
                match client.save_rules(&token, &bundle).await {
                    Ok(_) => {
                        raw_script.set(script_to_save);
                        rules.set(parsed);
                        status_msg.set(Some((
                            "Regras ManageSieve salvas com sucesso!".to_string(),
                            false,
                        )));
                    }
                    Err(e) => {
                        status_msg.set(Some((format!("Erro ao salvar: {}", e), true)));
                    }
                }
                is_loading.set(false);
            });
        }
    };

    let switch_to_visual = move |_| {
        let parsed = parse_sieve_script(&raw_script.read());
        rules.set(parsed);
        editor_mode.set(EditorMode::Visual);
    };

    let switch_to_hacker = move |_| {
        let generated = generate_sieve_script(&rules.read());
        raw_script.set(generated);
        editor_mode.set(EditorMode::Hacker);
    };

    let add_new_rule = move |_| {
        let mut curr = rules.read().clone();
        let new_id = format!("rule_{}", curr.len() + 1);
        curr.push(SieveRule {
            id: new_id,
            name: "Nova Regra".to_string(),
            enabled: true,
            match_all: true,
            conditions: vec![SieveCondition {
                field: ConditionField::From,
                match_type: MatchType::Contains,
                value: "".to_string(),
            }],
            actions: vec![
                SieveAction::FileInto("Pastas/Exemplo".to_string()),
                SieveAction::Stop,
            ],
        });
        rules.set(curr);
    };

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 1rem; max-width: 900px; margin: 0 auto; width: 100%;",

            // Barra Superior de Controle
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 0.8rem;",
                div {
                    style: "display: flex; align-items: center; gap: 0.8rem;",
                    button {
                        style: "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c;",
                        onclick: move |_| screen.set(Screen::Inbox),
                        "← VOLTAR"
                    }
                    span {
                        style: "font-weight: bold; font-size: 1.1rem; color: #0a0a0a;",
                        "REGRAS MANAGESIEVE"
                    }
                }

                // Alternador de Modo: Visual vs Hacker
                div {
                    style: "display: flex; gap: 0.4rem;",
                    button {
                        style: if *editor_mode.read() == EditorMode::Visual {
                            "background: #dc143c; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;"
                        } else {
                            "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.4rem 0.8rem; cursor: pointer;"
                        },
                        onclick: switch_to_visual,
                        "MODO VISUAL"
                    }
                    button {
                        style: if *editor_mode.read() == EditorMode::Hacker {
                            "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c;"
                        } else {
                            "background: #fff; color: #0a0a0a; border: 1px solid #0a0a0a; padding: 0.4rem 0.8rem; cursor: pointer;"
                        },
                        onclick: switch_to_hacker,
                        "HACKER (RAW)"
                    }
                    button {
                        style: "background: #2e7d32; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 1rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a;",
                        onclick: handle_save,
                        if *is_loading.read() { "SALVANDO..." } else { "💾 SALVAR" }
                    }
                }
            }

            // Mensagem de Status
            if let Some((ref msg, is_err)) = *status_msg.read() {
                div {
                    style: if is_err {
                        "border: 2px solid #dc143c; background: #ffebee; color: #b71c1c; padding: 0.6rem 1rem; font-weight: bold; font-size: 0.85rem;"
                    } else {
                        "border: 2px solid #2e7d32; background: #e8f5e9; color: #1b5e20; padding: 0.6rem 1rem; font-weight: bold; font-size: 0.85rem;"
                    },
                    "{msg}"
                }
            }

            // Conteúdo Conforme o Modo
            if *editor_mode.read() == EditorMode::Visual {
                div {
                    style: "display: flex; flex-direction: column; gap: 1rem;",

                    div {
                        style: "display: flex; justify-content: space-between; align-items: center;",
                        span { style: "font-weight: bold; font-size: 0.9rem; color: #555;", "REGRAS ATIVAS ({rules.read().len()})" }
                        button {
                            style: "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.3rem 0.7rem; font-size: 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c;",
                            onclick: add_new_rule,
                            "+ ADICIONAR REGRA"
                        }
                    }

                    if rules.read().is_empty() {
                        div {
                            style: "border: 2px dashed #0a0a0a; background: #fff; padding: 2rem; text-align: center; color: #666; font-style: italic;",
                            "Nenhuma regra configurada. Clique em '+ ADICIONAR REGRA' ou crie diretamente no leitor de mensagens."
                        }
                    }

                    for (idx, rule) in rules.read().iter().enumerate() {
                        {
                            let rule_name = rule.name.clone();
                            let rule_id = rule.id.clone();
                            let is_enabled = rule.enabled;

                            rsx! {
                                div {
                                    key: "{rule_id}",
                                    style: "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 3px 3px 0 #0a0a0a; display: flex; flex-direction: column; gap: 0.8rem;",

                                    div {
                                        style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #e0e0e0; padding-bottom: 0.5rem;",
                                        div {
                                            style: "display: flex; align-items: center; gap: 0.6rem;",
                                            span { style: "background: #0a0a0a; color: #fff; padding: 0.2rem 0.4rem; font-size: 0.75rem; font-weight: bold;", "#{idx + 1}" }
                                            input {
                                                style: "font-weight: bold; font-size: 1rem; border: 1px solid #ccc; padding: 0.2rem 0.4rem; font-family: monospace;",
                                                value: "{rule_name}",
                                                oninput: move |e| {
                                                    let mut curr = rules.read().clone();
                                                    if let Some(r) = curr.get_mut(idx) {
                                                        r.name = e.value();
                                                    }
                                                    rules.set(curr);
                                                }
                                            }
                                        }
                                        div {
                                            style: "display: flex; gap: 0.4rem;",
                                            button {
                                                style: if is_enabled {
                                                    "background: #e8f5e9; color: #2e7d32; border: 1px solid #2e7d32; padding: 0.2rem 0.5rem; font-size: 0.75rem; font-weight: bold; cursor: pointer;"
                                                } else {
                                                    "background: #ffebee; color: #c62828; border: 1px solid #c62828; padding: 0.2rem 0.5rem; font-size: 0.75rem; font-weight: bold; cursor: pointer;"
                                                },
                                                onclick: move |_| {
                                                    let mut curr = rules.read().clone();
                                                    if let Some(r) = curr.get_mut(idx) {
                                                        r.enabled = !r.enabled;
                                                    }
                                                    rules.set(curr);
                                                },
                                                if is_enabled { "ATIVADA" } else { "DESATIVADA" }
                                            }
                                            button {
                                                style: "background: #fff; color: #b71c1c; border: 1px solid #b71c1c; padding: 0.2rem 0.5rem; font-size: 0.75rem; cursor: pointer;",
                                                onclick: move |_| {
                                                    let mut curr = rules.read().clone();
                                                    if idx < curr.len() {
                                                        curr.remove(idx);
                                                        rules.set(curr);
                                                    }
                                                },
                                                "EXCLUIR"
                                            }
                                        }
                                    }

                                    // Condições (SE)
                                    div {
                                        style: "background: #fdfaf6; padding: 0.6rem; border-left: 3px solid #dc143c;",
                                        div { style: "font-size: 0.75rem; font-weight: bold; color: #dc143c; margin-bottom: 0.4rem;", "SE" }
                                        for (c_idx, cond) in rule.conditions.iter().enumerate() {
                                            {
                                                let cond_val = cond.value.clone();
                                                let cond_field_desc = match &cond.field {
                                                    ConditionField::From => "Remetente contém",
                                                    ConditionField::To => "Destinatário contém",
                                                    ConditionField::Subject => "Assunto contém",
                                                    ConditionField::ListId => "List-ID contém",
                                                    ConditionField::Custom(_h) => "Cabeçalho contém",
                                                };
                                                rsx! {
                                                    div {
                                                        key: "{c_idx}",
                                                        style: "display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.3rem;",
                                                        span { style: "font-size: 0.85rem; font-weight: bold; color: #333;", "{cond_field_desc}:" }
                                                        input {
                                                            style: "flex: 1; border: 1px solid #aaa; padding: 0.2rem 0.4rem; font-family: monospace; font-size: 0.85rem;",
                                                            value: "{cond_val}",
                                                            oninput: move |e| {
                                                                let mut curr = rules.read().clone();
                                                                if let Some(r) = curr.get_mut(idx)
                                                                    && let Some(c) = r.conditions.get_mut(c_idx)
                                                                {
                                                                    c.value = e.value();
                                                                }
                                                                rules.set(curr);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Ações (ENTÃO)
                                    div {
                                        style: "background: #fdfaf6; padding: 0.6rem; border-left: 3px solid #2e7d32;",
                                        div { style: "font-size: 0.75rem; font-weight: bold; color: #2e7d32; margin-bottom: 0.4rem;", "ENTÃO" }
                                        for action in &rule.actions {
                                            match action {
                                                SieveAction::FileInto(folder) => rsx! {
                                                    div {
                                                        style: "font-size: 0.85rem; color: #0a0a0a;",
                                                        "📁 Mover para pasta: "
                                                        span { style: "font-weight: bold;", "{folder}" }
                                                    }
                                                },
                                                SieveAction::MarkRead => rsx! {
                                                    div { style: "font-size: 0.85rem; color: #0a0a0a;", "✉ Marcar como lida" }
                                                },
                                                SieveAction::Discard => rsx! {
                                                    div { style: "font-size: 0.85rem; color: #b71c1c; font-weight: bold;", "🗑 Descartar mensagem (Discard)" }
                                                },
                                                SieveAction::Keep => rsx! {
                                                    div { style: "font-size: 0.85rem; color: #0a0a0a;", "📥 Manter na caixa de entrada (Keep)" }
                                                },
                                                SieveAction::Stop => rsx! {
                                                    div { style: "font-size: 0.85rem; color: #888; font-style: italic;", "⏹ Parar processamento de regras posteriores (Stop)" }
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Modo Hacker: Código Sieve Puro
                div {
                    style: "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; flex-direction: column; gap: 0.6rem;",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center;",
                        span { style: "font-weight: bold; font-size: 0.85rem; color: #dc143c;", "CÓDIGO SIEVE PURO (RFC 5228)" }
                        span { style: "font-size: 0.75rem; color: #888;", "Compatível com Dovecot Pigeonhole" }
                    }
                    textarea {
                        style: "width: 100%; min-height: 400px; font-family: monospace; font-size: 0.9rem; padding: 0.8rem; border: 1px solid #0a0a0a; background: #0a0a0a; color: #39ff14; box-sizing: border-box; resize: vertical;",
                        value: "{raw_script.read()}",
                        oninput: move |e| raw_script.set(e.value())
                    }
                }
            }
        }
    }
}
