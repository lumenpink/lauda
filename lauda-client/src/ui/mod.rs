use dioxus::prelude::*;

use crate::domain::view::ViewMode;

/// Componente raiz do smolclient Lauda com estética Trash Polka e alternador trifásico.
#[component]
pub fn App() -> Element {
    let mut current_view_mode = use_signal(|| ViewMode::TxtAuto);

    rsx! {
        div {
            style: "background-color: #f4efe6; color: #0a0a0a; font-family: monospace; min-height: 100vh; padding: 1.5rem;",
            header {
                style: "border-bottom: 3px solid #0a0a0a; padding-bottom: 1rem; margin-bottom: 1.5rem; display: flex; justify-content: space-between; align-items: center;",
                div {
                    h1 {
                        style: "font-size: 2.2rem; margin: 0; color: #0a0a0a;",
                        "LAUDA"
                    }
                    span {
                        style: "background: #dc143c; color: #fff; padding: 0.2rem 0.5rem; font-weight: bold; font-size: 0.8rem;",
                        "SMOLCLIENT"
                    }
                }
                div {
                    style: "display: flex; gap: 0.5rem;",
                    button {
                        style: if *current_view_mode.read() == ViewMode::TxtAuto { "background: #0a0a0a; color: #fff; padding: 0.4rem 0.8rem; border: none; font-weight: bold; cursor: pointer;" } else { "background: #fff; color: #0a0a0a; padding: 0.4rem 0.8rem; border: 1px solid #0a0a0a; cursor: pointer;" },
                        onclick: move |_| current_view_mode.set(ViewMode::TxtAuto),
                        "TXT AUTO"
                    }
                    button {
                        style: if *current_view_mode.read() == ViewMode::TxtOriginal { "background: #0a0a0a; color: #fff; padding: 0.4rem 0.8rem; border: none; font-weight: bold; cursor: pointer;" } else { "background: #fff; color: #0a0a0a; padding: 0.4rem 0.8rem; border: 1px solid #0a0a0a; cursor: pointer;" },
                        onclick: move |_| current_view_mode.set(ViewMode::TxtOriginal),
                        "TXT ORIGINAL"
                    }
                    button {
                        style: if *current_view_mode.read() == ViewMode::Html { "background: #0a0a0a; color: #fff; padding: 0.4rem 0.8rem; border: none; font-weight: bold; cursor: pointer;" } else { "background: #fff; color: #0a0a0a; padding: 0.4rem 0.8rem; border: 1px solid #0a0a0a; cursor: pointer;" },
                        onclick: move |_| current_view_mode.set(ViewMode::Html),
                        "HTML"
                    }
                }
            }
            main {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 1.5rem; box-shadow: 6px 6px 0 #0a0a0a;",
                div {
                    style: "margin-bottom: 1rem; font-weight: bold; color: #8b0000;",
                    {format!("MODO ATIVO: {:?}", *current_view_mode.read())}
                }
                p {
                    "Selecione uma mensagem na caixa de entrada para iniciar a leitura limpa."
                }
            }
        }
    }
}
