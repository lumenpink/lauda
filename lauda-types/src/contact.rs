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
    /// Data de aniversário (se disponível, ex: YYYY-MM-DD ou --MM-DD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday: Option<String>,
}

/// Registro completo de contato persistido e sincronizável via CardDAV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactEntry {
    /// Identificador único (UUID v4 canônico).
    pub id: String,
    /// Usuário proprietário do contato no Lauda.
    pub username: String,
    /// Nome completo de exibição.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// E-mail principal normalizado em minúsculas.
    pub primary_email: String,
    /// Lista de e-mails alternativos/secundários.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_emails: Vec<String>,
    /// Lista de telefones de contato.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phone_numbers: Vec<String>,
    /// Data de aniversário (RFC 6350 BDAY, ex: "YYYY-MM-DD" ou "--MM-DD").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birthday: Option<String>,
    /// Notas e observações livres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Payload cru vCard 4.0 (RFC 6350).
    pub vcard_raw: String,
    /// Indica se possui chave Autocrypt válida.
    pub has_autocrypt: bool,
    /// Fingerprint da chave pública Autocrypt (se disponível).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocrypt_fingerprint: Option<String>,
    /// ETag de integridade / hash de versão para HTTP e CardDAV.
    pub etag: String,
    /// Versão sequencial de modificação.
    pub version: u64,
    /// Timestamp Unix de criação.
    pub created_at: u64,
    /// Timestamp Unix da última modificação.
    pub updated_at: u64,
    /// Timestamp de soft-delete para sincronização de tombstones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<u64>,
}

impl ContactEntry {
    /// Calcula a similaridade entre dois registros para detecção de duplicatas (0.0 a 1.0).
    pub fn similarity_score(&self, other: &Self) -> f32 {
        // 1. Identificadores exatos de e-mail (Score 1.0)
        let s_email = self.primary_email.trim().to_lowercase();
        let o_email = other.primary_email.trim().to_lowercase();
        if !s_email.is_empty() && s_email == o_email {
            return 1.0;
        }

        // Match em e-mails secundários
        for e in &self.secondary_emails {
            let clean = e.trim().to_lowercase();
            if !clean.is_empty() && (clean == o_email || other.secondary_emails.iter().any(|oe| oe.trim().eq_ignore_ascii_case(&clean))) {
                return 1.0;
            }
        }

        // 2. Telefones normalizados coincidentes (Score 1.0)
        for sp in &self.phone_numbers {
            if let Ok(snorm) = crate::phone::PhoneNormalizer::normalize_br(sp, None) {
                for op in &other.phone_numbers {
                    if let Ok(onorm) = crate::phone::PhoneNormalizer::normalize_br(op, None)
                        && snorm.e164 == onorm.e164
                    {
                        return 1.0;
                    }
                }
            }
        }

        // 3. Similaridade de nome completo
        if let (Some(sn), Some(on)) = (&self.display_name, &other.display_name) {
            let sn_clean = sn.trim().to_lowercase();
            let on_clean = on.trim().to_lowercase();
            if !sn_clean.is_empty() && sn_clean == on_clean {
                return 0.9;
            }

            // Substring / nome contido
            if sn_clean.contains(&on_clean) || on_clean.contains(&sn_clean) {
                return 0.75;
            }
        }

        0.0
    }
}

/// Requisição de sincronização incremental.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ContactSyncRequest {
    /// Token de sincronização anterior (timestamp epoch ou etag sequencial).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_token: Option<String>,
    /// Limite de registros a retornar por página.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Resposta de sincronização incremental contendo alterações e tombstones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ContactSyncResponse {
    /// Novo token de sincronização a ser armazenado pelo cliente para a próxima chamada.
    pub sync_token: String,
    /// Contatos novos ou atualizados desde o último token.
    pub updated: Vec<ContactEntry>,
    /// IDs de contatos removidos desde o último token (tombstones).
    pub deleted_ids: Vec<String>,
    /// Indica se ainda há mais páginas disponíveis.
    pub has_more: bool,
}
