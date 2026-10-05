use serde::{Deserialize, Serialize};

/// Modos de visualização suportados pelo smolclient Lauda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    /// Renderização Markdown limpa gerada pelo pipeline do Urupema.
    #[default]
    TxtAuto,
    /// Texto cru original preservando quebras de linha e formatação ASCII original.
    TxtOriginal,
    /// Conteúdo HTML higienizado em sandbox.
    Html,
}

impl ViewMode {
    /// Alterna para o próximo modo no ciclo: TxtAuto -> Html -> TxtOriginal -> TxtAuto.
    pub fn next(&self) -> Self {
        match self {
            Self::TxtAuto => Self::Html,
            Self::Html => Self::TxtOriginal,
            Self::TxtOriginal => Self::TxtAuto,
        }
    }

    /// Rótulo conciso com glifo rotativo para botões compactos Trash Polka.
    pub fn label(&self) -> &'static str {
        match self {
            Self::TxtAuto => "TXT AUTO ⟳",
            Self::Html => "HTML ⟳",
            Self::TxtOriginal => "ORIGINAL ⟳",
        }
    }
}

/// Metadados resumidos de uma mensagem para listagens de caixa de entrada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Identificador estável IMAP (UID).
    pub uid: u32,
    /// Assunto da mensagem.
    pub subject: String,
    /// Remetente formatado ("Nome <email@dominio>").
    pub sender: String,
    /// Data no formato ISO-8601 UTC.
    pub date: String,
    /// Flag booleana de mensagem lida (\Seen).
    pub is_read: bool,
    /// Flag booleana de mensagem marcada (\Flagged).
    pub is_flagged: bool,
}

/// Mensagem completa incluindo envelopes e representações de corpo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Envelope e cabeçalhos principais da mensagem.
    #[serde(flatten)]
    pub envelope: MessageEnvelope,
    /// Versão Markdown limpa (txt auto).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txt_auto: Option<String>,
    /// Texto cru original preservado.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txt_original: Option<String>,
    /// HTML sanitizado sem scripts ou rastreadores.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_sanitized: Option<String>,
}
