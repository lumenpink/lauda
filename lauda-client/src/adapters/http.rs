use async_trait::async_trait;
use lauda_types::{
    ComposeForm, LoginRequest, MailboxInfo, Message, MessageEnvelope, TokenResponse,
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode};

use crate::domain::ports::{ClientError, MailApiClient};

/// Cliente HTTP real para consumo da API REST do Lauda Server.
#[derive(Debug, Clone)]
pub struct HttpApiClient {
    base_url: String,
    client: Client,
}

impl HttpApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: Client::builder().build().unwrap_or_default(),
        }
    }

    fn auth_headers(token: &str) -> Result<HeaderMap, ClientError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let auth_val = format!("Bearer {}", token);
        let header_val = HeaderValue::from_str(&auth_val)
            .map_err(|e| ClientError::Internal(format!("Token JWT inválido: {}", e)))?;
        headers.insert(AUTHORIZATION, header_val);
        Ok(headers)
    }
}

#[async_trait]
impl MailApiClient for HttpApiClient {
    async fn login(&self, req: &LoginRequest) -> Result<TokenResponse, ClientError> {
        let url = format!("{}/auth/login", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        match resp.status() {
            StatusCode::OK => resp
                .json::<TokenResponse>()
                .await
                .map_err(|e| ClientError::Network(format!("Erro ao deserializar login: {}", e))),
            StatusCode::UNAUTHORIZED => Err(ClientError::AuthFailed),
            status => Err(ClientError::Network(format!(
                "Status inesperado: {}",
                status
            ))),
        }
    }

    async fn list_mailboxes(&self, token: &str) -> Result<Vec<MailboxInfo>, ClientError> {
        let url = format!("{}/mailboxes", self.base_url);
        let headers = Self::auth_headers(token)?;
        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        match resp.status() {
            StatusCode::OK => resp.json::<Vec<MailboxInfo>>().await.map_err(|e| {
                ClientError::Network(format!("Erro ao deserializar caixas postais: {}", e))
            }),
            StatusCode::UNAUTHORIZED => Err(ClientError::AuthFailed),
            status => Err(ClientError::Network(format!(
                "Status inesperado: {}",
                status
            ))),
        }
    }

    async fn list_messages(
        &self,
        token: &str,
        mailbox: &str,
    ) -> Result<Vec<MessageEnvelope>, ClientError> {
        let url = format!("{}/messages?box={}", self.base_url, mailbox);
        let headers = Self::auth_headers(token)?;
        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        match resp.status() {
            StatusCode::OK => resp.json::<Vec<MessageEnvelope>>().await.map_err(|e| {
                ClientError::Network(format!("Erro ao deserializar mensagens: {}", e))
            }),
            StatusCode::UNAUTHORIZED => Err(ClientError::AuthFailed),
            status => Err(ClientError::Network(format!(
                "Status inesperado: {}",
                status
            ))),
        }
    }

    async fn get_message(
        &self,
        token: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<Message, ClientError> {
        let url = format!("{}/messages/{}?box={}", self.base_url, uid, mailbox);
        let headers = Self::auth_headers(token)?;
        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        match resp.status() {
            StatusCode::OK => resp
                .json::<Message>()
                .await
                .map_err(|e| ClientError::Network(format!("Erro ao deserializar mensagem: {}", e))),
            StatusCode::UNAUTHORIZED => Err(ClientError::AuthFailed),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound),
            status => Err(ClientError::Network(format!(
                "Status inesperado: {}",
                status
            ))),
        }
    }

    async fn send_message(&self, token: &str, form: &ComposeForm) -> Result<(), ClientError> {
        let url = format!("{}/compose", self.base_url);
        let headers = Self::auth_headers(token)?;
        let resp = self
            .client
            .post(&url)
            .headers(headers)
            .json(form)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        match resp.status() {
            StatusCode::OK | StatusCode::CREATED => Ok(()),
            StatusCode::UNAUTHORIZED => Err(ClientError::AuthFailed),
            status => Err(ClientError::Network(format!(
                "Erro ao enviar mensagem: {}",
                status
            ))),
        }
    }
}
