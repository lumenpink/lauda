use async_trait::async_trait;
use lauda_types::{ComposeForm, MailboxInfo, Message, MessageEnvelope};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Credenciais inválidas")]
    AuthFailed,
    #[error("Recurso não encontrado: {0}")]
    NotFound(String),
    #[error("Erro interno no backend de correio: {0}")]
    Internal(String),
}

/// Porta de abstração para o backend de e-mail (Dovecot IMAP / Sendmail ou In-Memory fake).
#[async_trait]
pub trait MailBackend: Send + Sync {
    async fn authenticate(&self, username: &str, password: &str) -> Result<(), BackendError>;
    async fn list_mailboxes(&self, username: &str) -> Result<Vec<MailboxInfo>, BackendError>;
    async fn list_messages(
        &self,
        username: &str,
        mailbox: &str,
    ) -> Result<Vec<MessageEnvelope>, BackendError>;
    async fn get_message(
        &self,
        username: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<Message, BackendError>;
    async fn send_message(&self, username: &str, form: &ComposeForm) -> Result<(), BackendError>;
    async fn mark_message_read(
        &self,
        username: &str,
        mailbox: &str,
        uid: u32,
        read: bool,
    ) -> Result<(), BackendError>;
    async fn delete_message(
        &self,
        username: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<(), BackendError>;
    async fn mark_message_flagged(
        &self,
        username: &str,
        mailbox: &str,
        uid: u32,
        flagged: bool,
    ) -> Result<(), BackendError>;
    async fn move_message(
        &self,
        username: &str,
        src_box: &str,
        dest_box: &str,
        uid: u32,
    ) -> Result<(), BackendError>;
    async fn get_attachment(
        &self,
        username: &str,
        mailbox: &str,
        uid: u32,
        attachment_id: usize,
    ) -> Result<(String, String, Vec<u8>), BackendError>;
    async fn list_contacts(
        &self,
        username: &str,
    ) -> Result<Vec<lauda_types::ContactInfo>, BackendError>;
    async fn get_sieve_script(&self, username: &str) -> Result<String, BackendError> {
        let _ = username;
        Ok(String::new())
    }
    async fn put_sieve_script(&self, username: &str, script: &str) -> Result<(), BackendError> {
        let _ = (username, script);
        Ok(())
    }
}
