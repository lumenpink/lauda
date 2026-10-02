use serde::{Deserialize, Serialize};

/// Requisição de login delegada ao Dovecot via IMAP pelo lauda-server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Resposta de sucesso contendo token JWT emitido pelo lauda-server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub expires_at: u64,
    pub username: String,
}
