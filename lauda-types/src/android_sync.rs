use serde::{Deserialize, Serialize};

use crate::contact::ContactEntry;
use crate::phone::PhoneNormalizer;

/// Tipo canônico de conta registrado pelo Lauda no AccountManager do Android.
pub const LAUDA_ACCOUNT_TYPE: &str = "pink.lumen.lauda";

/// Tipos MIME padronizados do ContactsProvider do Android.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AndroidMimeType {
    /// android.provider.ContactsContract.CommonDataKinds.StructuredName
    StructuredName,
    /// android.provider.ContactsContract.CommonDataKinds.Email
    Email,
    /// android.provider.ContactsContract.CommonDataKinds.Phone
    Phone,
    /// android.provider.ContactsContract.CommonDataKinds.Event (Tipo Birthday)
    EventBirthday,
    /// android.provider.ContactsContract.CommonDataKinds.Note
    Note,
}

impl AndroidMimeType {
    /// Retorna a string de identificação MIME oficial da plataforma Android.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StructuredName => "vnd.android.cursor.item/name",
            Self::Email => "vnd.android.cursor.item/email_v2",
            Self::Phone => "vnd.android.cursor.item/phone_v2",
            Self::EventBirthday => "vnd.android.cursor.item/contact_event",
            Self::Note => "vnd.android.cursor.item/note",
        }
    }
}

/// Operações atômicas de escrita contra o ContentResolver do Android.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AndroidRawContactOp {
    /// Criação de novo RawContact associado à conta gerenciada do Lauda.
    InsertRawContact {
        account_type: String,
        account_name: String,
        source_id: String,
    },
    /// Inserção de registro de dados (Data) atrelado ao RawContact pai.
    InsertData {
        mime_type: AndroidMimeType,
        value1: Option<String>,
        value2: Option<String>,
        value3: Option<String>,
        value4: Option<String>,
    },
    /// Remoção de contato (Tombstone reconciliado).
    DeleteRawContact {
        source_id: String,
    },
}

/// Lote atômico de operações preparado para execução via ContentResolver.applyBatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AndroidContactBatch {
    /// Tipo de conta do Android ("pink.lumen.lauda").
    pub account_type: String,
    /// Nome da conta (endereço da mailbox, ex: "hi@lumen.pink").
    pub account_name: String,
    /// Identificador único de origem (UUID do contato no Lauda).
    pub source_id: String,
    /// Sequência atômica de operações a executar em transação única no Android.
    pub operations: Vec<AndroidRawContactOp>,
}

impl AndroidContactBatch {
    /// Constrói um lote atômico de inserção/atualização a partir de um `ContactEntry`.
    pub fn from_contact_entry(entry: &ContactEntry, account_name: &str) -> Self {
        let mut operations = Vec::new();

        // 1. Inserção do RawContact mestre
        operations.push(AndroidRawContactOp::InsertRawContact {
            account_type: LAUDA_ACCOUNT_TYPE.to_string(),
            account_name: account_name.to_string(),
            source_id: entry.id.clone(),
        });

        // 2. Nome de exibição
        if let Some(ref name) = entry.display_name {
            operations.push(AndroidRawContactOp::InsertData {
                mime_type: AndroidMimeType::StructuredName,
                value1: Some(name.clone()),
                value2: None,
                value3: None,
                value4: None,
            });
        }

        // 3. E-mail principal
        if !entry.primary_email.is_empty() {
            operations.push(AndroidRawContactOp::InsertData {
                mime_type: AndroidMimeType::Email,
                value1: Some(entry.primary_email.clone()),
                value2: Some("1".to_string()), // TYPE_HOME / WORK
                value3: None,
                value4: None,
            });
        }

        // E-mails secundários
        for email in &entry.secondary_emails {
            operations.push(AndroidRawContactOp::InsertData {
                mime_type: AndroidMimeType::Email,
                value1: Some(email.clone()),
                value2: Some("2".to_string()),
                value3: None,
                value4: None,
            });
        }

        // 4. Telefones higienizados com PhoneNormalizer
        for phone in &entry.phone_numbers {
            let (formatted, e164) = match PhoneNormalizer::normalize_br(phone, None) {
                Ok(norm) => (norm.formatted, Some(norm.e164)),
                Err(_) => (phone.clone(), None),
            };

            operations.push(AndroidRawContactOp::InsertData {
                mime_type: AndroidMimeType::Phone,
                value1: Some(formatted),
                value2: Some("2".to_string()), // TYPE_MOBILE
                value3: None,
                value4: e164,
            });
        }

        // 5. Aniversário (RFC 6350 BDAY -> Event TYPE_BIRTHDAY)
        if let Some(ref bday) = entry.birthday {
            operations.push(AndroidRawContactOp::InsertData {
                mime_type: AndroidMimeType::EventBirthday,
                value1: Some(bday.clone()),
                value2: Some("3".to_string()), // TYPE_BIRTHDAY
                value3: None,
                value4: None,
            });
        }

        // 6. Notas
        if let Some(ref notes) = entry.notes {
            operations.push(AndroidRawContactOp::InsertData {
                mime_type: AndroidMimeType::Note,
                value1: Some(notes.clone()),
                value2: None,
                value3: None,
                value4: None,
            });
        }

        Self {
            account_type: LAUDA_ACCOUNT_TYPE.to_string(),
            account_name: account_name.to_string(),
            source_id: entry.id.clone(),
            operations,
        }
    }

    /// Constrói operação atômica de remoção para um tombstone sincronizado.
    pub fn delete_tombstone(source_id: &str, account_name: &str) -> Self {
        Self {
            account_type: LAUDA_ACCOUNT_TYPE.to_string(),
            account_name: account_name.to_string(),
            source_id: source_id.to_string(),
            operations: vec![AndroidRawContactOp::DeleteRawContact {
                source_id: source_id.to_string(),
            }],
        }
    }
}
