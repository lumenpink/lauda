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
}
