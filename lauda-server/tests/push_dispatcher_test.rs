use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use lauda_server::adapters::app_router;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_push_register_and_unregister_endpoints() {
    let backend = Arc::new(InMemoryMailBackend::new());
    let app = app_router(backend, "test-secret".to_string());

    // 1. Registrar endpoint
    let register_payload = json!({
        "endpoint": "https://push.example.org/endpoints/tok123",
        "instance": "lauda-test"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/push/register")
        .header("content-type", "application/json")
        .body(Body::from(register_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "POST /api/push/register deve retornar 200 OK"
    );

    // 2. Desregistrar endpoint
    let unregister_payload = json!({
        "endpoint": "https://push.example.org/endpoints/tok123"
    });

    let req = Request::builder()
        .method("DELETE")
        .uri("/api/push/unregister")
        .header("content-type", "application/json")
        .body(Body::from(unregister_payload.to_string()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "DELETE /api/push/unregister deve retornar 200 OK"
    );
}

#[tokio::test]
async fn test_push_dispatcher_triggers_blind_http_post() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use axum::routing::post;

    let received = Arc::new(AtomicBool::new(false));
    let received_clone = received.clone();

    // Servidor mock de distribuidor UnifiedPush
    let mock_app = axum::Router::new().route(
        "/mock-push-endpoint",
        post(move |body: String| {
            let r = received_clone.clone();
            async move {
                // Blind push: não pode conter credenciais nem corpos de e-mail sensíveis
                assert!(!body.contains("password") && !body.contains("Bearer"));
                r.store(true, Ordering::SeqCst);
                StatusCode::OK
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, mock_app).await.unwrap();
    });

    let endpoint_url = format!("http://{}/mock-push-endpoint", addr);

    let backend = Arc::new(InMemoryMailBackend::new());
    let app = app_router(backend, "test-secret".to_string());

    // Registrar no servidor
    let req = Request::builder()
        .method("POST")
        .uri("/api/push/register")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "endpoint": endpoint_url }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Disparar notificação de teste
    let req = Request::builder()
        .method("POST")
        .uri("/api/push/notify")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Aguardar entrega assíncrona
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(received.load(Ordering::SeqCst), "O webhook push deve ser entregue");
}
