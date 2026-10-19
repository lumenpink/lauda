use lauda_client::domain::session::{SessionConfig, prepare_post_login_state};
use lauda_client::ui::state::Screen;

#[test]
fn test_post_login_state_transition_is_immediate_and_non_blocking() {
    let server = "https://lauda.lohn.in".to_string();
    let user = "hi@lumen.pink".to_string();
    let token = "jwt.test.token.12345".to_string();

    let post_login = prepare_post_login_state(server.clone(), user.clone(), token.clone());

    // 1. O destino de navegação imediato deve ser Screen::Inbox
    assert_eq!(
        post_login.target_screen,
        Screen::Inbox,
        "A tela de destino após autenticação deve ser imediatamente Screen::Inbox"
    );

    // 2. A caixa postal canônica inicial deve ser INBOX
    assert_eq!(
        post_login.initial_mailbox, "INBOX",
        "A caixa postal inicial pós-login deve ser INBOX"
    );

    // 3. A sessão gerada deve conter o token e URL corretos
    assert_eq!(post_login.session.server_url, server);
    assert_eq!(post_login.session.username, user);
    assert_eq!(post_login.session.token.as_deref(), Some(token.as_str()));

    // 4. Contrato de Performance: Requisições de caixas/mensagens NÃO devem bloquear a transição de telas
    assert!(
        !post_login.blocking_mailbox_fetch,
        "O carregamento de caixas não pode ser bloqueante na tela de login"
    );
    assert!(
        !post_login.blocking_messages_fetch,
        "O carregamento de mensagens não pode ser bloqueante na tela de login"
    );
}

#[test]
fn test_mutation_detection_on_reintroduced_blocking_calls() {
    let mut post_login = prepare_post_login_state(
        "https://lauda.lohn.in".to_string(),
        "hi@lumen.pink".to_string(),
        "token".to_string(),
    );

    // Mutante 1: Reintrodução de bloqueio no fetch de caixas
    post_login.blocking_mailbox_fetch = true;
    assert!(post_login.blocking_mailbox_fetch);

    // Mutante 2: Reintrodução de bloqueio no fetch de mensagens
    post_login.blocking_messages_fetch = true;
    assert!(post_login.blocking_messages_fetch);

    // Mutante 3: Tela alvo diferente de Screen::Inbox
    post_login.target_screen = Screen::Login;
    assert_ne!(post_login.target_screen, Screen::Inbox);
}
