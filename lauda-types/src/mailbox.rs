use serde::{Deserialize, Serialize};

/// Informações sumárias sobre uma caixa de correio IMAP (ex: INBOX, Sent, Archive).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxInfo {
    pub name: String,
    pub total_messages: u32,
    pub unread_messages: u32,
}
