use lauda_server::adapters::maildir::is_sendmail_suppressed_for_tests;

#[test]
fn test_sendmail_suppression_flag_is_active_in_tests() {
    assert!(
        is_sendmail_suppressed_for_tests(),
        "O despacho de sendmail deve ser estritamente suprimido em contexto de testes"
    );
}
