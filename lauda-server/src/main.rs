use std::sync::Arc;

use lauda_server::adapters::InMemoryMailBackend;
use lauda_server::app_router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let jwt_secret = std::env::var("LAUDA_JWT_SECRET")
        .unwrap_or_else(|_| "lauda_default_development_secret_change_in_prod".to_string());
    let listen_addr =
        std::env::var("LAUDA_LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8432".to_string());

    let backend = Arc::new(InMemoryMailBackend::new());
    let app = app_router(backend, jwt_secret);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    tracing::info!("lauda-server escutando em {}", listen_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
