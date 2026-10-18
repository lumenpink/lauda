use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use std::sync::Mutex;
use tower::ServiceExt;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_spa_static_files_and_fallback_serving() {
    let _lock = ENV_LOCK.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let web_dir = tmp.path();

    std::fs::create_dir_all(web_dir.join("assets")).unwrap();
    std::fs::write(
        web_dir.join("index.html"),
        "<html><head><title>LAUDA SPA</title></head><body>SPA Root</body></html>",
    )
    .unwrap();
    std::fs::write(web_dir.join("assets/app.js"), "console.log('lauda');").unwrap();

    // Configurar variável de ambiente apontando para o diretório web de teste
    unsafe {
        std::env::set_var("LAUDA_WEB_DIR", web_dir);
    }

    let backend = Arc::new(InMemoryMailBackend::new());
    let app = app_router(backend, "test-jwt-secret-123".to_string());

    // 1. GET / -> Deve retornar index.html
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("LAUDA SPA"));

    // 2. GET /assets/app.js -> Deve retornar arquivo estático
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/assets/app.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("console.log('lauda');"));

    // 3. GET /inbox/view (rota SPA desconhecida no backend) -> Deve retornar index.html como fallback
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/inbox/view")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("LAUDA SPA"));

    // 4. GET /inbox?tab=unread (rota SPA com query string) -> Deve retornar index.html
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/inbox?tab=unread")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("LAUDA SPA"));

    // 5. POST /auth/login -> Rota da API não deve ser sobrescrita pelo fallback
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"test","password":"bad"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_spa_nonexistent_web_dir_falls_back_gracefully() {
    let _lock = ENV_LOCK.lock().unwrap();
    unsafe {
        std::env::set_var(
            "LAUDA_WEB_DIR",
            "/tmp/caminho_inexistente_para_web_spa_lauda_12345",
        );
    }

    let backend = Arc::new(InMemoryMailBackend::new());
    let app = app_router(backend, "test-jwt-secret-123".to_string());

    // Sem pasta estática existente, rotas desconhecidas retornam 404 sem panic
    let res = app
        .oneshot(
            Request::builder()
                .uri("/rota_inexistente")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

