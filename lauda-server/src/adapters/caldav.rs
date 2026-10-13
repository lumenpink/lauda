use axum::Router;
use axum::body::Bytes;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::header::HeaderMap;
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use lauda_types::calendar::{CalendarEvent, EventTime};

use crate::adapters::http::AppState;

/// Autenticação HTTP Basic ou Bearer para rotas CalDAV.
pub struct CalDavUser(pub String);

impl FromRequestParts<AppState> for CalDavUser {
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
                HeaderValue::from_static("Basic realm=\"Lauda CalDAV\""),
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
                return Ok(CalDavUser(user.to_string()));
            } else if let Some(token) = header.strip_prefix("Bearer ")
                && let Ok(user) = crate::auth::verify_token(token, &state.jwt_secret)
            {
                return Ok(CalDavUser(user));
            }
        }

        Err(unauthorized())
    }
}

/// Helper para resposta de OPTIONS com capacidades DAV/CalDAV anunciadas.
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
                "1, 3, calendar-access, access-control, sync-collection",
            ),
        ],
        "",
    )
        .into_response()
}

/// Redirecionamento da descoberta RFC 6764.
async fn well_known_caldav_handler() -> Response {
    (
        StatusCode::MOVED_PERMANENTLY,
        [(axum::http::header::LOCATION, "/dav/calendars/")],
        "Moved to CalDAV service",
    )
        .into_response()
}

/// PROPFIND / OPTIONS para /dav/calendars/
async fn calendars_root_handler(
    CalDavUser(username): CalDavUser,
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
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:response>
    <D:href>/dav/calendars/</D:href>
    <D:propstat>
      <D:prop>
        <D:current-user-principal><D:href>/dav/principals/{username}/</D:href></D:current-user-principal>
        <C:calendar-home-set><D:href>/dav/calendars/{username}/</D:href></C:calendar-home-set>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#
        )
    } else {
        format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:response>
    <D:href>/dav/calendars/{username}/default/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype>
          <D:collection/>
          <C:calendar/>
        </D:resourcetype>
        <C:supported-calendar-component-set>
          <C:comp name="VEVENT"/>
        </C:supported-calendar-component-set>
        <D:displayname>Agenda Lauda</D:displayname>
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

/// PROPFIND / OPTIONS para /dav/calendars/{username}/
async fn calendars_user_handler(
    CalDavUser(username): CalDavUser,
    Path(target_user): Path<String>,
    method: axum::http::Method,
) -> Response {
    if method == axum::http::Method::OPTIONS {
        return options_response();
    }
    if username != target_user {
        return StatusCode::FORBIDDEN.into_response();
    }

    let xml = format!(
        r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:response>
    <D:href>/dav/calendars/{username}/default/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype>
          <D:collection/>
          <C:calendar/>
        </D:resourcetype>
        <C:supported-calendar-component-set>
          <C:comp name="VEVENT"/>
        </C:supported-calendar-component-set>
        <D:displayname>Agenda Lauda</D:displayname>
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

/// REPORT / OPTIONS na coleção de calendário (/dav/calendars/{username}/default/)
async fn calendars_collection_report_handler(
    State(state): State<AppState>,
    CalDavUser(username): CalDavUser,
    Path(target_user): Path<String>,
    method: axum::http::Method,
    body: Bytes,
) -> Response {
    if method == axum::http::Method::OPTIONS {
        return options_response();
    }
    if username != target_user {
        return StatusCode::FORBIDDEN.into_response();
    }

    let store = match &state.calendar_store {
        Some(s) => s.clone(),
        None => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };

    let xml_payload = String::from_utf8_lossy(&body);

    if xml_payload.contains("calendar-query") {
        // Extrai time-range se especificado
        let mut start_ts = 0;
        let mut end_ts = i64::MAX;

        if let Some(pos) = xml_payload.find("time-range") {
            let slice = &xml_payload[pos..];
            if let Some(start_pos) = slice.find("start=\"") {
                let start_val = &slice[start_pos + 7..];
                if let Some(end_quote) = start_val.find('"')
                    && let Ok(et) = EventTime::from_ical_string(&start_val[..end_quote])
                {
                    start_ts = et.to_timestamp_utc();
                }
            }
            if let Some(end_pos) = slice.find("end=\"") {
                let end_val = &slice[end_pos + 5..];
                if let Some(end_quote) = end_val.find('"')
                    && let Ok(et) = EventTime::from_ical_string(&end_val[..end_quote])
                {
                    end_ts = et.to_timestamp_utc();
                }
            }
        }

        let events = match store.query_events_by_range(&username, start_ts, end_ts) {
            Ok(evs) => evs,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        let mut responses = String::new();
        for (id, ics_raw, etag) in events {
            responses.push_str(&format!(
                r#"  <D:response>
    <D:href>/dav/calendars/{username}/default/{id}.ics</D:href>
    <D:propstat>
      <D:prop>
        <D:getetag>"{etag}"</D:getetag>
        <C:calendar-data><![CDATA[{ics_raw}]]></C:calendar-data>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
"#
            ));
        }

        let out = format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
{responses}</D:multistatus>"#
        );

        return (
            StatusCode::MULTI_STATUS,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/xml; charset=utf-8",
            )],
            out,
        )
            .into_response();
    }

    if xml_payload.contains("sync-collection") {
        let since_token = extract_xml_tag_content(&xml_payload, "sync-token");
        let sync_res = match store.sync_events(&username, since_token.as_deref()) {
            Ok(res) => res,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        let mut responses = String::new();
        for (id, ics_raw, etag) in sync_res.updated {
            responses.push_str(&format!(
                r#"  <D:response>
    <D:href>/dav/calendars/{username}/default/{id}.ics</D:href>
    <D:propstat>
      <D:prop>
        <D:getetag>"{etag}"</D:getetag>
        <C:calendar-data><![CDATA[{ics_raw}]]></C:calendar-data>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
"#
            ));
        }

        for id in sync_res.deleted_ids {
            responses.push_str(&format!(
                r#"  <D:response>
    <D:href>/dav/calendars/{username}/default/{id}.ics</D:href>
    <D:status>HTTP/1.1 404 Not Found</D:status>
  </D:response>
"#
            ));
        }

        let out = format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
{responses}  <D:sync-token>{}</D:sync-token>
</D:multistatus>"#,
            sync_res.sync_token
        );

        return (
            StatusCode::MULTI_STATUS,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/xml; charset=utf-8",
            )],
            out,
        )
            .into_response();
    }

    if xml_payload.contains("calendar-multiget") {
        let hrefs = extract_all_xml_hrefs(&xml_payload);
        let mut responses = String::new();

        for href in hrefs {
            let id = href
                .rsplit('/')
                .next()
                .unwrap_or("")
                .trim_end_matches(".ics");
            if let Ok(Some((ics_raw, etag))) = store.get_event_raw(&username, id) {
                responses.push_str(&format!(
                    r#"  <D:response>
    <D:href>{href}</D:href>
    <D:propstat>
      <D:prop>
        <D:getetag>"{etag}"</D:getetag>
        <C:calendar-data><![CDATA[{ics_raw}]]></C:calendar-data>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
"#
                ));
            }
        }

        let out = format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
{responses}</D:multistatus>"#
        );

        return (
            StatusCode::MULTI_STATUS,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/xml; charset=utf-8",
            )],
            out,
        )
            .into_response();
    }

    StatusCode::BAD_REQUEST.into_response()
}

/// GET, PUT, DELETE para /dav/calendars/{username}/default/{id}
async fn calendar_item_handler(
    State(state): State<AppState>,
    CalDavUser(username): CalDavUser,
    Path((target_user, raw_id)): Path<(String, String)>,
    method: axum::http::Method,
    body: Bytes,
) -> Response {
    if username != target_user {
        return StatusCode::FORBIDDEN.into_response();
    }

    let store = match &state.calendar_store {
        Some(s) => s.clone(),
        None => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };

    let id = raw_id.trim_end_matches(".ics");

    if method == axum::http::Method::GET {
        match store.get_event_raw(&username, id) {
            Ok(Some((ics_raw, etag))) => {
                let mut res = (
                    StatusCode::OK,
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "text/calendar; charset=utf-8",
                    )],
                    ics_raw,
                )
                    .into_response();
                if let Ok(hv) = HeaderValue::from_str(&format!("\"{etag}\"")) {
                    res.headers_mut().insert(axum::http::header::ETAG, hv);
                }
                res
            }
            Ok(None) => StatusCode::NOT_FOUND.into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    } else if method == axum::http::Method::PUT {
        let raw_str = String::from_utf8_lossy(&body);
        let event = match CalendarEvent::from_ical(&raw_str) {
            Ok(ev) => ev,
            Err(_) => {
                // Fallback de contingência caso venha formato simplificado
                CalendarEvent {
                    uid: id.to_string(),
                    summary: "Evento".to_string(),
                    description: None,
                    location: None,
                    start: EventTime::DateTimeUtc(0),
                    end: EventTime::DateTimeUtc(0),
                    is_all_day: false,
                    rrule: None,
                    status: lauda_types::calendar::EventStatus::Confirmed,
                    organizer: None,
                    attendees: Vec::new(),
                    sequence: 0,
                    created_at: None,
                    updated_at: None,
                }
            }
        };

        match store.upsert_event(&username, &event) {
            Ok(etag) => {
                let mut res = StatusCode::CREATED.into_response();
                if let Ok(hv) = HeaderValue::from_str(&format!("\"{etag}\"")) {
                    res.headers_mut().insert(axum::http::header::ETAG, hv);
                }
                res
            }
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    } else if method == axum::http::Method::DELETE {
        match store.delete_event(&username, id) {
            Ok(true) => StatusCode::NO_CONTENT.into_response(),
            Ok(false) => StatusCode::NOT_FOUND.into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    } else {
        StatusCode::METHOD_NOT_ALLOWED.into_response()
    }
}

fn extract_xml_tag_content(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let open_ns = format!("<D:{tag}>");
    let close = format!("</{tag}>");
    let close_ns = format!("</D:{tag}>");

    let start_idx = xml
        .find(&open)
        .map(|i| i + open.len())
        .or_else(|| xml.find(&open_ns).map(|i| i + open_ns.len()))?;

    let remainder = &xml[start_idx..];
    let end_idx = remainder
        .find(&close)
        .or_else(|| remainder.find(&close_ns))?;

    Some(remainder[..end_idx].trim().to_string())
}

fn extract_all_xml_hrefs(xml: &str) -> Vec<String> {
    let mut hrefs = Vec::new();
    let mut pos = 0;
    while let Some(start) = xml[pos..].find("<D:href>") {
        let abs_start = pos + start + 8;
        if let Some(end) = xml[abs_start..].find("</D:href>") {
            let href = xml[abs_start..abs_start + end].trim().to_string();
            hrefs.push(href);
            pos = abs_start + end + 9;
        } else {
            break;
        }
    }
    hrefs
}

/// Constrói o roteador Axum das rotas CalDAV.
pub fn caldav_routes() -> Router<AppState> {
    Router::new()
        .route("/.well-known/caldav", get(well_known_caldav_handler))
        .route(
            "/dav/calendars/",
            axum::routing::any(calendars_root_handler),
        )
        .route(
            "/dav/calendars/{username}/",
            axum::routing::any(calendars_user_handler),
        )
        .route(
            "/dav/calendars/{username}/default/",
            axum::routing::any(calendars_collection_report_handler),
        )
        .route(
            "/dav/calendars/{username}/default/{id}",
            axum::routing::any(calendar_item_handler),
        )
}
