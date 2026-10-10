use lauda_client::domain::{ActionType, ActionsQueue};
use tempfile::tempdir;

#[test]
fn test_actions_queue_lifecycle() {
    let temp = tempdir().expect("tempdir");
    let queue = ActionsQueue::new(temp.path());

    // Inicialmente vazia
    assert_eq!(queue.count().unwrap(), 0);
    assert_eq!(queue.list().unwrap().len(), 0);

    // Enfileirar marcação como lido
    let a1 = queue
        .enqueue(101, "INBOX", ActionType::MarkRead { read: true }, 1000)
        .expect("enqueue 1");
    assert_eq!(queue.count().unwrap(), 1);
    assert_eq!(a1.uid, 101);
    assert_eq!(a1.action, ActionType::MarkRead { read: true });

    // Enfileirar marcação como favorito
    let a2 = queue
        .enqueue(102, "INBOX", ActionType::MarkFlagged { flagged: true }, 1005)
        .expect("enqueue 2");
    assert_eq!(queue.count().unwrap(), 2);

    let list = queue.list().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, a1.id);
    assert_eq!(list[1].id, a2.id);

    // Remoção da ação 1 após sync
    assert!(queue.remove(&a1.id).unwrap());
    assert_eq!(queue.count().unwrap(), 1);

    // Persistência entre reinicializações
    let reloaded = ActionsQueue::new(temp.path());
    assert_eq!(reloaded.count().unwrap(), 1);
    let rem_list = reloaded.list().unwrap();
    assert_eq!(rem_list[0].id, a2.id);
}
