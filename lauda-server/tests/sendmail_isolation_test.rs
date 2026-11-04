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
