use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::adapters::sqlite_contacts::SqliteContactsStore;
use lauda_server::app_router_with_contacts;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_carddav_full_lifecycle_and_sync() {
    let temp_dir = tempfile::tempdir().expect("Criar tempdir");
    let db_path = temp_dir.path().join("contacts.db");
    let contacts_store = SqliteContactsStore::open(&db_path).expect("Abrir sqlite");

    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("lumen", "senha123");

    let app = app_router_with_contacts(backend, "test-secret", contacts_store);

    let auth_header = format!(
        "Basic {}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, "lumen:senha123")
    );

    // 0. Autenticação: Requisição não autorizada deve retornar 401
    let req = Request::builder()
        .uri("/dav/addressbooks/")
        .method("PROPFIND")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert!(resp.headers().contains_key("www-authenticate"));

    // 1. Well-known redirect
    let req = Request::builder()
        .uri("/.well-known/carddav")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        resp.headers().get("location").unwrap(),
        "/dav/addressbooks/"
    );

    // 2. OPTIONS no root e na coleção deve anunciar capacidades DAV
    let req = Request::builder()
        .uri("/dav/addressbooks/")
        .method("OPTIONS")
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(
        resp.headers()
            .get("dav")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("addressbook")
    );

    // 3. PROPFIND no principal / home-set
    let req = Request::builder()
        .uri("/dav/addressbooks/")
        .method("PROPFIND")
        .header("Authorization", &auth_header)
        .header("Depth", "0")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_xml = String::from_utf8_lossy(&body_bytes);
    assert!(body_xml.contains("addressbook-home-set"));

    // 4. PUT de um vCard
    let vcard_content = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:c1-carddav-uuid\r\nFN:Maria Firmina\r\nEMAIL:maria@letras.org\r\nEND:VCARD\r\n";
    let req = Request::builder()
        .uri("/dav/addressbooks/lumen/default/c1-carddav-uuid.vcf")
        .method("PUT")
        .header("Authorization", &auth_header)
        .header("Content-Type", "text/vcard; charset=utf-8")
        .body(Body::from(vcard_content))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(resp.status() == StatusCode::CREATED || resp.status() == StatusCode::NO_CONTENT);
    assert!(resp.headers().contains_key("etag"));

    // 5. GET do vCard
    let req = Request::builder()
        .uri("/dav/addressbooks/lumen/default/c1-carddav-uuid.vcf")
        .method("GET")
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_vcard = String::from_utf8_lossy(&body_bytes);
    assert!(body_vcard.contains("Maria Firmina"));

    // 6. REPORT sync-collection
    let sync_req_xml = "<?xml version=\"1.0\" encoding=\"utf-8\" ?>\n<sync-collection xmlns=\"DAV:\"><sync-token/><sync-level>1</sync-level></sync-collection>";
    let req = Request::builder()
        .uri("/dav/addressbooks/lumen/default/")
        .method("REPORT")
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/xml")
        .body(Body::from(sync_req_xml))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_xml = String::from_utf8_lossy(&body_bytes);
    assert!(body_xml.contains("c1-carddav-uuid.vcf"));
    assert!(body_xml.contains("sync-token"));

    // 7. DELETE do vCard
    let req = Request::builder()
        .uri("/dav/addressbooks/lumen/default/c1-carddav-uuid.vcf")
        .method("DELETE")
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // 8. GET após DELETE deve retornar 404
    let req = Request::builder()
        .uri("/dav/addressbooks/lumen/default/c1-carddav-uuid.vcf")
        .method("GET")
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // 9. REPORT sync-collection após DELETE deve conter tombstone 404
    let req = Request::builder()
        .uri("/dav/addressbooks/lumen/default/")
        .method("REPORT")
        .header("Authorization", &auth_header)
        .header("Content-Type", "application/xml")
        .body(Body::from(sync_req_xml))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_xml = String::from_utf8_lossy(&body_bytes);
    assert!(body_xml.contains("c1-carddav-uuid.vcf"));
    assert!(body_xml.contains("404 Not Found"));
}
