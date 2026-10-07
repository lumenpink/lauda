use serde::{Deserialize, Serialize};

/// Dados de anexo enviado pelo compositor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentUpload {
    /// Nome original do arquivo.
    pub filename: String,
    /// Tipo MIME do anexo.
    pub content_type: String,
    /// Conteúdo bruto codificado em Base64.
    pub data_base64: String,
}

/// Formulário de composição e redação rápida sem travas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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
    /// Anexos adicionados na redação.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentUpload>,
}
