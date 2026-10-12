use axum::Router;
use axum::body::Bytes;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::header::HeaderMap;
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use lauda_types::contact::{ContactEntry, ContactSyncRequest};

use crate::adapters::http::AppState;

/// Autenticação HTTP Basic ou Bearer para rotas CardDAV.
pub struct CardDavUser(pub String);

impl FromRequestParts<AppState> for CardDavUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        let unauthorized = || {
            let mut res = (StatusCode::UNAUTHORIZED, "Autenticação requerida").into_response();
            res.headers_mut().insert(
                axum::http::header::WWW_AUTHENTICATE,
                HeaderValue::from_static("Basic realm=\"Lauda CardDAV\""),
            );
            res
        };

        if let Some(header) = auth_header {
            if let Some(basic_b64) = header.strip_prefix("Basic ")
                && let Ok(decoded) =
                    base64::engine::general_purpose::STANDARD.decode(basic_b64.trim())
                && let Ok(user_pass) = String::from_utf8(decoded)
                && let Some((user, pass)) = user_pass.split_once(':')
                && state.backend.authenticate(user, pass).await.is_ok()
            {
                return Ok(CardDavUser(user.to_string()));
            } else if let Some(token) = header.strip_prefix("Bearer ")
                && let Ok(user) = crate::auth::verify_token(token, &state.jwt_secret)
            {
                return Ok(CardDavUser(user));
            }
        }

        Err(unauthorized())
    }
}

/// Helper determinístico FNV-1a para geração de ETags de vCards.
fn compute_etag(content: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in content {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// Helper para resposta de OPTIONS com capacidades DAV/CardDAV anunciadas.
fn options_response() -> Response {
    (
        StatusCode::OK,
        [
            (
                axum::http::header::ALLOW,
                "OPTIONS, GET, HEAD, POST, PUT, DELETE, PROPFIND, REPORT",
            ),
            (
                axum::http::HeaderName::from_static("dav"),
                "1, 3, addressbook, access-control, sync-collection",
            ),
        ],
        "",
    )
        .into_response()
}

/// Redirecionamento da descoberta RFC 6764.
async fn well_known_carddav_handler() -> Response {
    (
        StatusCode::MOVED_PERMANENTLY,
        [(axum::http::header::LOCATION, "/dav/addressbooks/")],
        "Moved to CardDAV service",
    )
        .into_response()
}

/// PROPFIND / OPTIONS para /dav/addressbooks/
async fn addressbooks_root_handler(
    CardDavUser(username): CardDavUser,
    method: axum::http::Method,
    headers: HeaderMap,
) -> Response {
    if method == axum::http::Method::OPTIONS {
        return options_response();
    }
    if method.as_str() != "PROPFIND" && method != axum::http::Method::GET {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }

    let depth = headers
        .get("Depth")
        .and_then(|d| d.to_str().ok())
        .unwrap_or("0");

    let xml = if depth == "0" {
        format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:carddav">
  <D:response>
    <D:href>/dav/addressbooks/</D:href>
    <D:propstat>
      <D:prop>
        <D:current-user-principal><D:href>/dav/principals/{username}/</D:href></D:current-user-principal>
        <C:addressbook-home-set><D:href>/dav/addressbooks/{username}/</D:href></C:addressbook-home-set>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#
        )
    } else {
        format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:carddav">
  <D:response>
    <D:href>/dav/addressbooks/{username}/default/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype>
          <D:collection/>
          <C:addressbook/>
        </D:resourcetype>
        <D:displayname>Contatos Lauda</D:displayname>
        <D:sync-token>ts:0</D:sync-token>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#
        )
    };

    (
        StatusCode::MULTI_STATUS,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/xml; charset=utf-8",
        )],
        xml,
    )
        .into_response()
}

/// PROPFIND / OPTIONS para /dav/addressbooks/{username}/
async fn addressbooks_user_handler(
    CardDavUser(username): CardDavUser,
    method: axum::http::Method,
    _headers: HeaderMap,
) -> Response {
    if method == axum::http::Method::OPTIONS {
        return options_response();
    }
    if method.as_str() != "PROPFIND" && method != axum::http::Method::GET {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }

    let xml = format!(
        r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:carddav">
  <D:response>
    <D:href>/dav/addressbooks/{username}/default/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype>
          <D:collection/>
          <C:addressbook/>
        </D:resourcetype>
        <D:displayname>Contatos Lauda</D:displayname>
        <D:sync-token>ts:0</D:sync-token>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#
    );

    (
        StatusCode::MULTI_STATUS,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/xml; charset=utf-8",
        )],
        xml,
    )
        .into_response()
}

/// Handler da coleção de contatos: PROPFIND, REPORT e OPTIONS para /dav/addressbooks/{username}/default/
async fn addressbook_collection_handler(
    State(state): State<AppState>,
    CardDavUser(username): CardDavUser,
    method: axum::http::Method,
    body: Bytes,
) -> Response {
    match method.as_str() {
        "OPTIONS" => options_response(),
        "PROPFIND" => {
            let xml = format!(
                r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:carddav">
  <D:response>
    <D:href>/dav/addressbooks/{username}/default/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype>
          <D:collection/>
          <C:addressbook/>
        </D:resourcetype>
        <D:displayname>Contatos Lauda</D:displayname>
        <D:sync-token>ts:0</D:sync-token>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#
            );

            (
                StatusCode::MULTI_STATUS,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/xml; charset=utf-8",
                )],
                xml,
            )
                .into_response()
        }
        "REPORT" => {
            let store = match &state.contacts_store {
                Some(s) => s.clone(),
                None => return StatusCode::NOT_IMPLEMENTED.into_response(),
            };

            let body_str = String::from_utf8_lossy(&body);

            // Extrair sync-token se presente no body XML
            let sync_token = if body_str.contains("<sync-token>") {
                body_str
                    .split("<sync-token>")
                    .nth(1)
                    .and_then(|s| s.split("</sync-token>").next())
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
            } else {
                None
            };

            let sync_req = ContactSyncRequest {
                sync_token,
                limit: Some(500),
            };

            let sync_resp = match store.sync_contacts(&username, &sync_req) {
                Ok(r) => r,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };

            let mut responses_xml = String::new();
            for contact in sync_resp.updated {
                let href = format!("/dav/addressbooks/{username}/default/{}.vcf", contact.id);
                responses_xml.push_str(&format!(
                    r#"
  <D:response>
    <D:href>{href}</D:href>
    <D:propstat>
      <D:prop>
        <D:getetag>"{}"</D:getetag>
        <C:address-data xmlns:C="urn:ietf:params:xml:ns:carddav"><![CDATA[{}]]></C:address-data>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>"#,
                    contact.etag, contact.vcard_raw
                ));
            }

            for deleted_id in sync_resp.deleted_ids {
                let href = format!("/dav/addressbooks/{username}/default/{deleted_id}.vcf");
                responses_xml.push_str(&format!(
                    r#"
  <D:response>
    <D:href>{href}</D:href>
    <D:status>HTTP/1.1 404 Not Found</D:status>
  </D:response>"#
                ));
            }

            let xml = format!(
                r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:carddav">
  {responses_xml}
  <D:sync-token>{}</D:sync-token>
</D:multistatus>"#,
                sync_resp.sync_token
            );

            (
                StatusCode::MULTI_STATUS,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/xml; charset=utf-8",
                )],
                xml,
            )
                .into_response()
        }
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

/// Handler para vCard individual: GET, PUT, DELETE, OPTIONS para /dav/addressbooks/{username}/default/{filename}
async fn vcard_item_handler(
    State(state): State<AppState>,
    CardDavUser(username): CardDavUser,
    Path((_user, filename)): Path<(String, String)>,
    method: axum::http::Method,
    body: Bytes,
) -> Response {
    let store = match &state.contacts_store {
        Some(s) => s.clone(),
        None => return StatusCode::NOT_IMPLEMENTED.into_response(),
    };

    let id = filename.trim_end_matches(".vcf");

    match method.as_str() {
        "OPTIONS" => options_response(),
        "GET" => match store.get_contact_by_id(&username, id) {
            Ok(Some(c)) => (
                StatusCode::OK,
                [
                    (
                        axum::http::header::CONTENT_TYPE,
                        "text/vcard; charset=utf-8",
                    ),
                    (axum::http::header::ETAG, &format!("\"{}\"", c.etag)),
                ],
                c.vcard_raw,
            )
                .into_response(),
            Ok(None) => StatusCode::NOT_FOUND.into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
        "PUT" => {
            let vcard_str = match String::from_utf8(body.to_vec()) {
                Ok(s) => s,
                Err(_) => return StatusCode::BAD_REQUEST.into_response(),
            };

            let mut display_name = None;
            let mut primary_email = format!("{}@unknown.local", id);

            for line in vcard_str.lines() {
                let trimmed = line.trim();
                if let Some(fn_val) = trimmed.strip_prefix("FN:") {
                    display_name = Some(fn_val.trim().to_string());
                } else if let Some(em_val) = trimmed.strip_prefix("EMAIL:") {
                    primary_email = em_val.trim().to_string();
                } else if let Some(em_val) = trimmed.strip_prefix("EMAIL;")
                    && let Some((_, email_part)) = em_val.split_once(':')
                {
                    primary_email = email_part.trim().to_string();
                }
            }

            let now_ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let etag = compute_etag(vcard_str.as_bytes());

            let entry = ContactEntry {
                id: id.to_string(),
                username,
                display_name,
                primary_email,
                secondary_emails: vec![],
                phone_numbers: vec![],
                notes: None,
                vcard_raw: vcard_str,
                has_autocrypt: false,
                autocrypt_fingerprint: None,
                etag: etag.clone(),
                version: 1,
                created_at: now_ts,
                updated_at: now_ts,
                deleted_at: None,
            };

            if store.upsert_contact(&entry).is_ok() {
                (
                    StatusCode::CREATED,
                    [(axum::http::header::ETAG, format!("\"{}\"", etag))],
                )
                    .into_response()
            } else {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
        "DELETE" => {
            let now_ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            match store.delete_contact(&username, id, now_ts) {
                Ok(true) => StatusCode::NO_CONTENT.into_response(),
                Ok(false) => StatusCode::NOT_FOUND.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

/// Monta o roteador com os endpoints CardDAV padrão integrados ao AppState.
pub fn carddav_routes() -> Router<AppState> {
    Router::new()
        .route("/.well-known/carddav", get(well_known_carddav_handler))
        .route(
            "/dav/addressbooks/",
            axum::routing::any(addressbooks_root_handler),
        )
        .route(
            "/dav/addressbooks/{username}/",
            axum::routing::any(addressbooks_user_handler),
        )
        .route(
            "/dav/addressbooks/{username}/default/",
            axum::routing::any(addressbook_collection_handler),
        )
        .route(
            "/dav/addressbooks/{username}/default/{filename}",
            axum::routing::any(vcard_item_handler),
        )
}
