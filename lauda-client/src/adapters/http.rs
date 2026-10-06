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

    async fn mark_read(
        &self,
        token: &str,
        mailbox: &str,
        uid: u32,
        read: bool,
    ) -> Result<(), ClientError> {
        let url = format!(
            "{}/messages/{}/read?box={}&read={}",
            self.base_url, uid, mailbox, read
        );
        let headers = Self::auth_headers(token)?;
        let resp = self
            .client
            .patch(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        match resp.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED => Err(ClientError::AuthFailed),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound),
            status => Err(ClientError::Network(format!(
                "Erro ao alterar status de leitura: {}",
                status
            ))),
        }
    }

    async fn delete_message(
        &self,
        token: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<(), ClientError> {
        let url = format!("{}/messages/{}?box={}", self.base_url, uid, mailbox);
        let headers = Self::auth_headers(token)?;
        let resp = self
            .client
            .delete(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        match resp.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED => Err(ClientError::AuthFailed),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound),
            status => Err(ClientError::Network(format!(
                "Erro ao excluir mensagem: {}",
                status
            ))),
        }
    }

    async fn mark_flagged(
        &self,
        token: &str,
        mailbox: &str,
        uid: u32,
        flagged: bool,
    ) -> Result<(), ClientError> {
        let url = format!(
            "{}/messages/{}/flag?box={}&flagged={}",
            self.base_url, uid, mailbox, flagged
        );
        let headers = Self::auth_headers(token)?;
        let resp = self
            .client
            .patch(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        match resp.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED => Err(ClientError::AuthFailed),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound),
            status => Err(ClientError::Network(format!(
                "Erro ao alterar status de favorito: {}",
                status
            ))),
        }
    }

    async fn move_message(
        &self,
        token: &str,
        uid: u32,
        src_box: &str,
        dest_box: &str,
    ) -> Result<(), ClientError> {
        let url = format!(
            "{}/messages/{}/move?src={}&dest={}",
            self.base_url, uid, src_box, dest_box
        );
        let headers = Self::auth_headers(token)?;
        let resp = self
            .client
            .post(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        match resp.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED => Err(ClientError::AuthFailed),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound),
            status => Err(ClientError::Network(format!(
                "Erro ao mover mensagem: {}",
                status
            ))),
        }
    }

    fn attachment_url(&self, mailbox: &str, uid: u32, attachment_id: usize) -> String {
        format!(
            "{}/messages/{}/attachments/{}?box={}",
            self.base_url, uid, attachment_id, mailbox
        )
    }

    async fn fetch_attachment(
        &self,
        token: &str,
        mailbox: &str,
        uid: u32,
        attachment_id: usize,
    ) -> Result<(String, String, Vec<u8>), ClientError> {
        let url = self.attachment_url(mailbox, uid, attachment_id);
        let headers = Self::auth_headers(token)?;
        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ClientError::Network(e.to_string()))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            return Err(ClientError::AuthFailed);
        }
        if resp.status() == StatusCode::NOT_FOUND {
            return Err(ClientError::NotFound);
        }
        if !resp.status().is_success() {
            return Err(ClientError::Network(format!(
                "Erro ao baixar anexo: status {}",
                resp.status()
            )));
        }

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let filename = resp
            .headers()
            .get(reqwest::header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .and_then(|cd| {
                if let Some(pos) = cd.find("filename=\"") {
                    let rest = &cd[pos + 10..];
                    rest.split('"').next()
                } else if let Some(pos) = cd.find("filename=") {
                    let rest = &cd[pos + 9..];
                    rest.split(';').next()
                } else {
                    None
                }
            })
            .unwrap_or("anexo")
            .to_string();

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ClientError::Network(format!("Erro ao ler bytes do anexo: {}", e)))?
            .to_vec();

        Ok((filename, content_type, bytes))
    }
}
