use async_trait::async_trait;
use lauda_types::{
    ComposeForm, LoginRequest, MailboxInfo, Message, MessageEnvelope, TokenResponse,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Falha de autenticação")]
    AuthFailed,
    #[error("Erro de rede / requisição: {0}")]
    Network(String),
    #[error("Recurso não encontrado")]
    NotFound,
    #[error("Erro interno no cliente: {0}")]
    Internal(String),
}

/// Porta de comunicação com a API remota do Lauda.
#[async_trait]
pub trait MailApiClient: Send + Sync {
    async fn login(&self, req: &LoginRequest) -> Result<TokenResponse, ClientError>;
    async fn list_mailboxes(&self, token: &str) -> Result<Vec<MailboxInfo>, ClientError>;
    async fn list_messages(
        &self,
        token: &str,
        mailbox: &str,
    ) -> Result<Vec<MessageEnvelope>, ClientError>;
    async fn get_message(
        &self,
        token: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<Message, ClientError>;
    async fn send_message(&self, token: &str, form: &ComposeForm) -> Result<(), ClientError>;
    async fn mark_read(
        &self,
        token: &str,
        mailbox: &str,
        uid: u32,
        read: bool,
    ) -> Result<(), ClientError>;
    async fn mark_flagged(
        &self,
        token: &str,
        mailbox: &str,
        uid: u32,
        flagged: bool,
    ) -> Result<(), ClientError>;
    async fn move_message(
        &self,
        token: &str,
        uid: u32,
        src_box: &str,
        dest_box: &str,
    ) -> Result<(), ClientError>;
    fn attachment_url(&self, mailbox: &str, uid: u32, attachment_id: usize) -> String;
    async fn fetch_attachment(
        &self,
        token: &str,
        mailbox: &str,
        uid: u32,
        attachment_id: usize,
    ) -> Result<(String, String, Vec<u8>), ClientError>;
    async fn delete_message(&self, token: &str, mailbox: &str, uid: u32)
    -> Result<(), ClientError>;
    async fn list_contacts(
        &self,
        token: &str,
    ) -> Result<Vec<lauda_types::ContactInfo>, ClientError>;
}
