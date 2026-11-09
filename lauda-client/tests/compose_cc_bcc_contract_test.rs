use lauda_client::domain::outbox::OutboxQueue;
use lauda_types::ComposeForm;
use tempfile::tempdir;

#[test]
fn test_compose_form_bcc_outbox_queue_contract() {
    let temp = tempdir().unwrap();
    let queue = OutboxQueue::new(temp.path());

    let form = ComposeForm {
        from: "Lumen <hi@lumen.pink>".to_string(),
        to: "amigo@exemplo.org".to_string(),
        cc: Some("copia@exemplo.org".to_string()),
        bcc: Some("segredo@exemplo.org".to_string()),
        subject: "Teste com CCO".to_string(),
        body: "Corpo do e-mail".to_string(),
        attachments: vec![],
    };

    let item = queue.enqueue(form.clone(), 12345).expect("enqueue com bcc");
    assert_eq!(item.form.bcc, Some("segredo@exemplo.org".to_string()));
    assert_eq!(item.form.cc, Some("copia@exemplo.org".to_string()));

    let list = queue.list().expect("list queue");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].form.bcc, Some("segredo@exemplo.org".to_string()));
}

#[test]
fn test_compose_view_source_code_contract() {
    let source_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/compose_view.rs");
    let content = std::fs::read_to_string(source_path).expect("compose_view.rs deve existir");

    // 1. Deve conter lógica condicional de expansão baseada em reply_cc
    assert!(
        content.contains("reply_cc.as_ref().is_some_and(|c| !c.trim().is_empty())"),
        "Deve inicializar show_cc_bcc expandido apenas se reply_cc contiver valor"
    );

    // 2. Deve conter botão para expansão sob demanda
    assert!(
        content.contains("+ CÓPIA (CC / CCO)"),
        "Deve exibir botão '+ CÓPIA (CC / CCO)' quando recolhido"
    );

    // 3. Deve conter rótulos devidamente traduzidos
    assert!(
        content.contains("CÓPIA (CC):"),
        "Deve conter campo 'CÓPIA (CC):'"
    );
    assert!(
        content.contains("CÓPIA OCULTA (CCO):"),
        "Deve conter campo 'CÓPIA OCULTA (CCO):'"
    );

    // 4. Botão de expansão deve ter tabindex -1 para não atrapalhar navegação Tab
    assert!(
        content.contains("tabindex: \"-1\""),
        "Botão de alternância de cópia deve ter tabindex -1 para fluxo desimpedido via Tab"
    );
}
