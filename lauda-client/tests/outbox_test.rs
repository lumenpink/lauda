use lauda_client::domain::{OutboxQueue, OutboxStatus};
use lauda_types::ComposeForm;
use tempfile::tempdir;

#[test]
fn test_outbox_queue_lifecycle() {
    let temp = tempdir().expect("tempdir");
    let queue = OutboxQueue::new(temp.path());

    // Inicialmente vazia
    assert_eq!(queue.list().unwrap().len(), 0);
    assert_eq!(queue.pending_count().unwrap(), 0);

    let form1 = ComposeForm {
        from: "lumen@2lp.in".to_string(),
        to: "alice@example.com".to_string(),
        cc: None,
        subject: "Primeiro Teste".to_string(),
        body: "Texto do teste 1".to_string(),
        attachments: vec![],
    };

    let item1 = queue.enqueue(form1.clone(), 1000).expect("enqueue 1");
    assert_eq!(item1.status, OutboxStatus::Pending);
    assert_eq!(item1.retry_count, 0);
    assert_eq!(queue.pending_count().unwrap(), 1);

    let form2 = ComposeForm {
        from: "lumen@2lp.in".to_string(),
        to: "bob@example.com".to_string(),
        cc: None,
        subject: "Segundo Teste".to_string(),
        body: "Texto do teste 2".to_string(),
        attachments: vec![],
    };

    let item2 = queue.enqueue(form2.clone(), 1005).expect("enqueue 2");
    assert_eq!(queue.pending_count().unwrap(), 2);

    // Listagem deve conter os dois itens
    let list = queue.list().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, item1.id);
    assert_eq!(list[1].id, item2.id);

    // Atualiza status do item 1 para Falha com incremento de retry
    queue
        .update_status(&item1.id, OutboxStatus::Failed("Erro de conexão".to_string()), true)
        .expect("update status");

    let updated_list = queue.list().unwrap();
    let updated_item1 = updated_list.iter().find(|i| i.id == item1.id).unwrap();
    assert_eq!(updated_item1.retry_count, 1);
    assert_eq!(
        updated_item1.status,
        OutboxStatus::Failed("Erro de conexão".to_string())
    );

    // Remoção do item 1 após envio bem-sucedido
    assert!(queue.remove(&item1.id).unwrap());
    assert_eq!(queue.pending_count().unwrap(), 1);

    // Persistência entre reaberturas da fila
    let reloaded_queue = OutboxQueue::new(temp.path());
    let reloaded_list = reloaded_queue.list().unwrap();
    assert_eq!(reloaded_list.len(), 1);
    assert_eq!(reloaded_list[0].id, item2.id);
}
