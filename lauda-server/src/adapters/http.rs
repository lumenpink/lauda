use std::sync::Arc;

use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use lauda_types::{
    ComposeForm, ContactInfo, LoginRequest, MailboxInfo, Message, MessageEnvelope, TokenResponse,
};
use serde::Deserialize;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::auth::{create_token, verify_token};
use crate::domain::{BackendError, MailBackend};

#[derive(Clone)]
pub struct AppState {
    pub backend: Arc<dyn MailBackend>,
    pub jwt_secret: String,
    pub events_tx: tokio::sync::broadcast::Sender<String>,
    pub contacts_store: Option<Arc<crate::adapters::sqlite_contacts::SqliteContactsStore>>,
    pub calendar_store: Option<Arc<crate::adapters::sqlite_calendar::SqliteCalendarStore>>,
}

pub struct AuthUser(pub String);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = if let Some(auth_header) = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
        {
            auth_header
                .strip_prefix("Bearer ")
                .ok_or((StatusCode::UNAUTHORIZED, "Formato do token inválido"))?
        } else if let Some(query) = parts.uri.query() {
            query
                .split('&')
                .find_map(|pair| pair.strip_prefix("token="))
                .ok_or((
                    StatusCode::UNAUTHORIZED,
                    "Header de autorização ou token ausente",
                ))?
        } else {
            return Err((StatusCode::UNAUTHORIZED, "Header de autorização ausente"));
        };

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
    let _ = state.events_tx.send(format!(
        "{{\"event\":\"read_update\",\"box\":\"{}\",\"uid\":{}}}",
        mailbox, uid
    ));
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
    let _ = state.events_tx.send(format!(
        "{{\"event\":\"delete\",\"box\":\"{}\",\"uid\":{}}}",
        mailbox, uid
    ));
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct MarkFlagQuery {
    pub r#box: Option<String>,
    pub flagged: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct MoveMessageQuery {
    pub src: Option<String>,
    pub dest: String,
}

async fn mark_flag_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
    Path(uid): Path<u32>,
    Query(query): Query<MarkFlagQuery>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mailbox = query.r#box.as_deref().unwrap_or("INBOX");
    let flagged = query.flagged.unwrap_or(true);
    state
        .backend
        .mark_message_flagged(&username, mailbox, uid, flagged)
        .await
        .map_err(|e| match e {
            BackendError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;
    let _ = state.events_tx.send(format!(
        "{{\"event\":\"flag_update\",\"box\":\"{}\",\"uid\":{}}}",
        mailbox, uid
    ));
    Ok(StatusCode::NO_CONTENT)
}

async fn move_message_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
    Path(uid): Path<u32>,
    Query(query): Query<MoveMessageQuery>,
) -> Result<StatusCode, (StatusCode, String)> {
    let src = query.src.as_deref().unwrap_or("INBOX");
    state
        .backend
        .move_message(&username, src, &query.dest, uid)
        .await
        .map_err(|e| match e {
            BackendError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;
    let _ = state.events_tx.send(format!(
        "{{\"event\":\"move\",\"src\":\"{}\",\"dest\":\"{}\",\"uid\":{}}}",
        src, query.dest, uid
    ));
    Ok(StatusCode::NO_CONTENT)
}

async fn get_attachment_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
    Path((uid, att_id)): Path<(u32, usize)>,
    Query(query): Query<MessagesQuery>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let mailbox = query.r#box.as_deref().unwrap_or("INBOX");
    let (filename, content_type, bytes) = state
        .backend
        .get_attachment(&username, mailbox, uid, att_id)
        .await
        .map_err(|e| match e {
            BackendError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;

    let mut response = axum::response::Response::new(axum::body::Body::from(bytes));
    let headers = response.headers_mut();
    if let Ok(val) = axum::http::HeaderValue::from_str(&content_type) {
        headers.insert(axum::http::header::CONTENT_TYPE, val);
    }
    let disp = format!("attachment; filename=\"{}\"", filename);
    if let Ok(val) = axum::http::HeaderValue::from_str(&disp) {
        headers.insert(axum::http::header::CONTENT_DISPOSITION, val);
    }
    Ok(response)
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
    let _ = state
        .events_tx
        .send("{\"event\":\"sent\",\"box\":\"Sent\"}".to_string());
    Ok(StatusCode::CREATED)
}

async fn events_handler(
    State(state): State<AppState>,
    AuthUser(_username): AuthUser,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.events_tx.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|res| res.ok())
        .map(|msg| Ok(Event::default().data(msg)));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn list_contacts_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
) -> Result<Json<Vec<ContactInfo>>, (StatusCode, String)> {
    let contacts = state
        .backend
        .list_contacts(&username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(contacts))
}

async fn sync_contacts_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
    Json(payload): Json<lauda_types::contact::ContactSyncRequest>,
) -> Result<Json<lauda_types::contact::ContactSyncResponse>, (StatusCode, String)> {
    let store = state.contacts_store.as_ref().ok_or((
        StatusCode::NOT_IMPLEMENTED,
        "Contacts store não configurado".to_string(),
    ))?;
    let resp = store
        .sync_contacts(&username, &payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(resp))
}

async fn get_rules_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
) -> Result<Json<lauda_types::sieve::SieveScriptBundle>, (StatusCode, String)> {
    let script = state
        .backend
        .get_sieve_script(&username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let rules = lauda_types::sieve::parse_sieve_script(&script);
    Ok(Json(lauda_types::sieve::SieveScriptBundle {
        raw_script: script,
        rules,
    }))
}

async fn put_rules_handler(
    State(state): State<AppState>,
    AuthUser(username): AuthUser,
    Json(payload): Json<lauda_types::sieve::SieveScriptBundle>,
) -> Result<StatusCode, (StatusCode, String)> {
    let script = if payload.raw_script.trim().is_empty() {
        lauda_types::sieve::generate_sieve_script(&payload.rules)
    } else {
        payload.raw_script
    };

    state
        .backend
        .put_sieve_script(&username, &script)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn app_router(backend: Arc<dyn MailBackend>, jwt_secret: impl Into<String>) -> Router {
    let (events_tx, _) = tokio::sync::broadcast::channel(128);
    let state = AppState {
        backend,
        jwt_secret: jwt_secret.into(),
        events_tx,
        contacts_store: None,
        calendar_store: None,
    };

    build_router(state)
}

pub fn app_router_with_contacts(
    backend: Arc<dyn MailBackend>,
    jwt_secret: impl Into<String>,
    contacts_store: crate::adapters::sqlite_contacts::SqliteContactsStore,
) -> Router {
    let (events_tx, _) = tokio::sync::broadcast::channel(128);
    let state = AppState {
        backend,
        jwt_secret: jwt_secret.into(),
        events_tx,
        contacts_store: Some(Arc::new(contacts_store)),
        calendar_store: None,
    };

    build_router(state)
}

pub fn app_router_with_calendar(
    backend: Arc<dyn MailBackend>,
    jwt_secret: impl Into<String>,
    contacts_store: crate::adapters::sqlite_contacts::SqliteContactsStore,
    calendar_store: crate::adapters::sqlite_calendar::SqliteCalendarStore,
) -> Router {
    let (events_tx, _) = tokio::sync::broadcast::channel(128);
    let state = AppState {
        backend,
        jwt_secret: jwt_secret.into(),
        events_tx,
        contacts_store: Some(Arc::new(contacts_store)),
        calendar_store: Some(Arc::new(calendar_store)),
    };

    build_router(state)
}

fn build_router(state: AppState) -> Router {
    let base = Router::new()
        .route("/auth/login", post(login_handler))
        .route("/events", get(events_handler))
        .route("/contacts", get(list_contacts_handler))
        .route("/contacts/sync", post(sync_contacts_handler))
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
        .route(
            "/messages/{uid}/flag",
            patch(mark_flag_handler).post(mark_flag_handler),
        )
        .route("/messages/{uid}/move", post(move_message_handler))
        .route(
            "/messages/{uid}/attachments/{id}",
            get(get_attachment_handler),
        )
        .route("/rules", get(get_rules_handler).put(put_rules_handler))
        .route("/compose", post(compose_handler))
        .layer(tower_http::cors::CorsLayer::permissive());

    let carddav = crate::adapters::carddav::carddav_routes();
    let caldav = crate::adapters::caldav::caldav_routes();

    base.merge(carddav).merge(caldav).with_state(state)
}
