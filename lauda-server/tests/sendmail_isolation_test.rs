use lauda_server::adapters::maildir::is_sendmail_suppressed_for_tests;

#[test]
fn test_sendmail_suppression_flag_is_active_in_tests() {
    assert!(
        is_sendmail_suppressed_for_tests(),
        "O despacho de sendmail deve ser estritamente suprimido em contexto de testes"
    );
}

#[test]
fn test_smtp_live_dispatch_is_disabled_by_default() {
    // Na ausência de variáveis de ambiente permissivas, is_smtp_live_dispatch_enabled DEVE ser falso
    assert!(
        !lauda_server::adapters::maildir::is_smtp_live_dispatch_enabled(),
        "O envio SMTP real deve ser estritamente desabilitado por padrão (segurança opt-in)"
    );
}

#[tokio::test]
async fn test_dispatch_email_suppressed_by_default_without_network() {
    // Sem LAUDA_SMTP_ENABLED e sem mock, o despacho não deve abrir conexão e deve retornar Ok(())
    let res = lauda_server::adapters::maildir::MaildirBackend::dispatch_email(
        "test@example.test",
        &["dest@example.test".to_string()],
        "From: test@example.test\nTo: dest@example.test\nSubject: Test\n\nBody",
    )
    .await;

    assert!(
        res.is_ok(),
        "dispatch_email deve retornar Ok(()) e suprimir envio por padrão"
    );
}

#[test]
fn test_mutation_coverage_smtp_live_dispatch_flags() {
    // 1. Testa que "0" ou "false" explicitamente NÃO ativam
    unsafe {
        std::env::set_var("LAUDA_SMTP_ENABLED", "0");
    }
    assert!(!lauda_server::adapters::maildir::is_smtp_live_dispatch_enabled());

    unsafe {
        std::env::set_var("LAUDA_SMTP_ENABLED", "false");
    }
    assert!(!lauda_server::adapters::maildir::is_smtp_live_dispatch_enabled());

    unsafe {
        std::env::set_var("LAUDA_SMTP_ENABLED", "random_string");
    }
    assert!(!lauda_server::adapters::maildir::is_smtp_live_dispatch_enabled());

    // 2. Testa que mesmo se LAUDA_SMTP_ENABLED=true estiver definido, uma flag de teste como LAUDA_TEST_MODE desativa
    unsafe {
        std::env::set_var("LAUDA_SMTP_ENABLED", "true");
        std::env::set_var("LAUDA_TEST_MODE", "1");
    }
    assert!(
        !lauda_server::adapters::maildir::is_smtp_live_dispatch_enabled(),
        "LAUDA_TEST_MODE deve suprimir o envio mesmo com LAUDA_SMTP_ENABLED=true"
    );

    // 3. Testa que CI=true também desativa
    unsafe {
        std::env::remove_var("LAUDA_TEST_MODE");
        std::env::set_var("CI", "true");
    }
    assert!(
        !lauda_server::adapters::maildir::is_smtp_live_dispatch_enabled(),
        "CI deve suprimir o envio mesmo com LAUDA_SMTP_ENABLED=true"
    );

    // 4. Limpeza sanitária
    unsafe {
        std::env::remove_var("LAUDA_SMTP_ENABLED");
        std::env::remove_var("CI");
    }
    assert!(!lauda_server::adapters::maildir::is_smtp_live_dispatch_enabled());
}
