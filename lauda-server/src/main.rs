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

    let maildir_env = std::env::var("LAUDA_MAILDIR_PATH").ok();
    let default_maildir =
        std::path::PathBuf::from("/home/lumen/stacks/mail/data/dms/mail-data/lumen.pink/hi");
    let maildir_path = maildir_env.map(std::path::PathBuf::from).or_else(|| {
        if default_maildir.exists() {
            Some(default_maildir)
        } else {
            None
        }
    });

    let backend: Arc<dyn lauda_server::domain::ports::MailBackend> = if let Some(path) =
        maildir_path
    {
        tracing::info!("lauda-server: inicializando MaildirBackend em {:?}", path);
        let m = Arc::new(lauda_server::adapters::MaildirBackend::new(path));
        m.add_user("lumen", "senha123");
        m.add_user("hi@lumen.pink", "senha123");
        m
    } else {
        tracing::info!("lauda-server: inicializando InMemoryMailBackend de demonstração");
        let m = Arc::new(InMemoryMailBackend::new());
        m.add_user("lumen", "senha123");
        m.add_mailbox(
            "lumen",
            lauda_types::MailboxInfo {
                name: "INBOX".to_string(),
                total_messages: 2,
                unread_messages: 1,
            },
        );
        m.add_message(
            "lumen",
            "INBOX",
            lauda_types::Message {
                envelope: lauda_types::MessageEnvelope {
                    uid: 101,
                    subject: "Bem-vindo ao Lauda Smolclient".to_string(),
                    sender: "postmaster@2lp.in".to_string(),
                    date: "2026-10-04T12:00:00Z".to_string(),
                    is_read: false,
                    is_flagged: false,
                },
                txt_auto: Some("# Bem-vindo ao Lauda Smolclient\n\nEste é o cliente de e-mail minimalista em texto puro com suporte a Dioxus 0.7 e Rust 2024.\n\n- Visualização limpa\n- Sem JavaScript em execução\n- Alta eficiência energética".to_string()),
                txt_original: Some("Bem-vindo ao Lauda Smolclient\n\nEste é o cliente de e-mail minimalista em texto puro com suporte a Dioxus 0.7 e Rust 2024.".to_string()),
                html_sanitized: Some("<div style='font-family:sans-serif;'><h3>Bem-vindo ao Lauda</h3><p>Cliente de e-mail seguro.</p></div>".to_string()),
            },
        );
        m.add_message(
            "lumen",
            "INBOX",
            lauda_types::Message {
                envelope: lauda_types::MessageEnvelope {
                    uid: 102,
                    subject: "Atualização de Segurança e Criptografia".to_string(),
                    sender: "security@2lp.in".to_string(),
                    date: "2026-10-04T13:30:00Z".to_string(),
                    is_read: true,
                    is_flagged: false,
                },
                txt_auto: Some("## Relatório de Segurança\n\nChaves Autocrypt verificadas com sucesso para lumen@2lp.in.".to_string()),
                txt_original: Some("Relatório de Segurança\n\nChaves Autocrypt verificadas com sucesso para lumen@2lp.in.".to_string()),
                html_sanitized: Some("<p>Chaves Autocrypt verificadas.</p>".to_string()),
            },
        );
        m
    };
    let app = app_router(backend, jwt_secret);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    tracing::info!("lauda-server escutando em {}", listen_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
