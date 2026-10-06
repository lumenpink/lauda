use std::sync::Arc;

use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use lauda_types::{
    ComposeForm, LoginRequest, MailboxInfo, Message, MessageEnvelope, TokenResponse,
};
use serde::Deserialize;

use crate::auth::{create_token, verify_token};
use crate::domain::{BackendError, MailBackend};

#[derive(Clone)]
pub struct AppState {
    pub backend: Arc<dyn MailBackend>,
    pub jwt_secret: String,
}

pub struct AuthUser(pub String);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Header de autorização ausente"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, "Formato do token inválido"))?;

        let username = verify_token(token, &state.jwt_secret)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Token inválido ou expirado"))?;

        Ok(AuthUser(username))
    }
}

#[derive(Debug, Deserialize)]
pub struct MessagesQuery {
    pub r#box: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MarkReadQuery {
    pub r#box: Option<String>,
    pub read: Option<bool>,
}

async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, (StatusCode, &'static str)> {
    state
        .backend
        .authenticate(&payload.username, &payload.password)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Credenciais inválidas"))?;

    let (token, expires_at) =
        create_token(&payload.username, &state.jwt_secret, 72).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Falha ao gerar token JWT",
            )
        })?;

    Ok(Json(TokenResponse {
        token,
        expires_at,
        username: payload.username,
    }))
}

async fn list_mailboxes_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
) -> Result<Json<Vec<MailboxInfo>>, (StatusCode, String)> {
    let boxes = state
        .backend
        .list_mailboxes(&username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(boxes))
}

async fn list_messages_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
    Query(query): Query<MessagesQuery>,
) -> Result<Json<Vec<MessageEnvelope>>, (StatusCode, String)> {
    let mailbox = query.r#box.as_deref().unwrap_or("INBOX");
    let msgs = state
        .backend
        .list_messages(&username, mailbox)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(msgs))
}

async fn get_message_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
    Path(uid): Path<u32>,
    Query(query): Query<MessagesQuery>,
) -> Result<Json<Message>, (StatusCode, String)> {
    let mailbox = query.r#box.as_deref().unwrap_or("INBOX");
    let msg = state
        .backend
        .get_message(&username, mailbox, uid)
        .await
        .map_err(|e| match e {
            BackendError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;
    Ok(Json(msg))
}

async fn mark_read_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
    Path(uid): Path<u32>,
    Query(query): Query<MarkReadQuery>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mailbox = query.r#box.as_deref().unwrap_or("INBOX");
    let read = query.read.unwrap_or(true);
    state
        .backend
        .mark_message_read(&username, mailbox, uid, read)
        .await
        .map_err(|e| match e {
            BackendError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_message_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
    Path(uid): Path<u32>,
    Query(query): Query<MessagesQuery>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mailbox = query.r#box.as_deref().unwrap_or("INBOX");
    state
        .backend
        .delete_message(&username, mailbox, uid)
        .await
        .map_err(|e| match e {
            BackendError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn compose_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
    Json(form): Json<ComposeForm>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .backend
        .send_message(&username, &form)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::CREATED)
}

pub fn app_router(backend: Arc<dyn MailBackend>, jwt_secret: impl Into<String>) -> Router {
    let state = AppState {
        backend,
        jwt_secret: jwt_secret.into(),
    };

    Router::new()
        .route("/auth/login", post(login_handler))
        .route("/mailboxes", get(list_mailboxes_handler))
        .route("/messages", get(list_messages_handler))
        .route(
            "/messages/{uid}",
            get(get_message_handler).delete(delete_message_handler),
        )
        .route(
            "/messages/{uid}/read",
            patch(mark_read_handler).post(mark_read_handler),
        )
        .route("/compose", post(compose_handler))
        .with_state(state)
}
