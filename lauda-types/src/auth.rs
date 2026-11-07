use serde::{Deserialize, Serialize};

/// Requisição de login delegada ao Dovecot via IMAP pelo lauda-server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_type: Option<String>,
}

impl LoginRequest {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            client_type: None,
        }
    }

    pub fn with_client_type(mut self, client_type: impl Into<String>) -> Self {
        self.client_type = Some(client_type.into());
        self
    }
}

/// Resposta de sucesso contendo token JWT emitido pelo lauda-server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub expires_at: u64,
    pub username: String,
}
