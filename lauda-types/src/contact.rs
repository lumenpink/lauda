use serde::{Deserialize, Serialize};

/// Informações resumidas de um contato conhecido pelo MUA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ContactInfo {
    /// Endereço de e-mail do contato.
    pub email: String,
    /// Nome de exibição (se disponível).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Número de mensagens trocadas registradas.
    pub message_count: usize,
    /// Indica se o contato possui chave Autocrypt ativa para criptografia ponta-a-ponta.
    pub has_autocrypt: bool,
    /// Fingerprint da chave pública Autocrypt (se disponível).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocrypt_fingerprint: Option<String>,
}
