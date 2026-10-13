use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::adapters::sqlite_calendar::SqliteCalendarStore;
use lauda_server::adapters::sqlite_contacts::SqliteContactsStore;
use lauda_server::app_router_with_calendar;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_caldav_full_lifecycle_and_sync() {
    let temp_dir = tempfile::tempdir().expect("Criar tempdir");
    let contacts_db = temp_dir.path().join("contacts.db");
    let calendar_db = temp_dir.path().join("calendar.db");

    let contacts_store = SqliteContactsStore::open(&contacts_db).expect("Abrir contacts sqlite");
    let calendar_store = SqliteCalendarStore::open(&calendar_db).expect("Abrir calendar sqlite");

    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("lumen", "senha123");

    let app = app_router_with_calendar(backend, "test-secret", contacts_store, calendar_store);

    let auth_header = format!(
        "Basic {}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, "lumen:senha123")
    );

    // 0. Autenticação: 401 sem credenciais
    let req = Request::builder()
        .uri("/dav/calendars/")
        .method("PROPFIND")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert!(resp.headers().contains_key("www-authenticate"));

    // 1. Well-known discovery redirect
    let req = Request::builder()
        .uri("/.well-known/caldav")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(resp.headers().get("location").unwrap(), "/dav/calendars/");

    // 2. OPTIONS no root deve anunciar calendar-access
    let req = Request::builder()
        .uri("/dav/calendars/")
        .method("OPTIONS")
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let dav_header = resp
        .headers()
        .get("dav")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(dav_header.contains("calendar-access"));

    // 3. PROPFIND Depth: 0 no root
    let req = Request::builder()
        .uri("/dav/calendars/")
        .method("PROPFIND")
        .header("Authorization", &auth_header)
        .header("Depth", "0")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("/dav/principals/lumen/"));
    assert!(body_str.contains("/dav/calendars/lumen/"));

    // 4. PROPFIND Depth: 1 no usuário
    let req = Request::builder()
        .uri("/dav/calendars/lumen/")
        .method("PROPFIND")
        .header("Authorization", &auth_header)
        .header("Depth", "1")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("<C:calendar"));
    assert!(body_str.contains("Agenda Lauda"));

    // 5. PUT: Criação de evento VEVENT
    let sample_ics = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//Lauda//PT-BR\r\n\
BEGIN:VEVENT\r\n\
UID:event-test-42\r\n\
SUMMARY:Almoço com Marcelo\r\n\
LOCATION:Fasano\r\n\
DTSTART:20261014T123000Z\r\n\
DTEND:20261014T133000Z\r\n\
STATUS:CONFIRMED\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    let req = Request::builder()
        .uri("/dav/calendars/lumen/default/event-test-42.ics")
        .method("PUT")
        .header("Authorization", &auth_header)
        .header("Content-Type", "text/calendar; charset=utf-8")
        .body(Body::from(sample_ics))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success());
    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();

    // 6. GET: Leitura do evento salvo
    let req = Request::builder()
        .uri("/dav/calendars/lumen/default/event-test-42.ics")
        .method("GET")
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("etag").unwrap(), &etag);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("SUMMARY:Almoço com Marcelo"));
    assert!(body_str.contains("LOCATION:Fasano"));

    // 7. REPORT: calendar-query com filtro time-range
    let query_xml = r#"<?xml version="1.0" encoding="utf-8" ?>
<C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop>
    <D:getetag/>
    <C:calendar-data/>
  </D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:comp-filter name="VEVENT">
        <C:time-range start="20261014T000000Z" end="20261014T235959Z"/>
      </C:comp-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#;

    let req = Request::builder()
        .uri("/dav/calendars/lumen/default/")
        .method("REPORT")
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/xml")
        .body(Body::from(query_xml))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("/dav/calendars/lumen/default/event-test-42.ics"));
    assert!(body_str.contains("Almoço com Marcelo"));

    // 8. REPORT: sync-collection inicial
    let sync_xml = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:sync-collection xmlns:D="DAV:">
  <D:sync-token>ts:0</D:sync-token>
  <D:prop>
    <D:getetag/>
  </D:prop>
</D:sync-collection>"#;

    let req = Request::builder()
        .uri("/dav/calendars/lumen/default/")
        .method("REPORT")
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/xml")
        .body(Body::from(sync_xml))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("/dav/calendars/lumen/default/event-test-42.ics"));
    assert!(body_str.contains("<D:sync-token>"));

    // 9. DELETE: Remoção suave do evento
    let req = Request::builder()
        .uri("/dav/calendars/lumen/default/event-test-42.ics")
        .method("DELETE")
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // 10. REPORT: sync-collection após exclusão deve conter tombstone 404
    let req = Request::builder()
        .uri("/dav/calendars/lumen/default/")
        .method("REPORT")
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/xml")
        .body(Body::from(sync_xml))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("/dav/calendars/lumen/default/event-test-42.ics"));
    assert!(body_str.contains("HTTP/1.1 404 Not Found"));
}
