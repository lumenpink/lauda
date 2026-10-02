use serde::{Deserialize, Serialize};

/// Formulário de composição e redação rápida sem travas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposeForm {
    /// Remetente livre (pode ser alias configurado).
    pub from: String,
    /// Destinatário principal.
    pub to: String,
    /// Cópia carbono (opcional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<String>,
    /// Assunto da mensagem.
    pub subject: String,
    /// Corpo da mensagem em fonte monoespaçada.
    pub body: String,
}
