use lauda_client::domain::view::merge_search_results;
use lauda_types::MessageEnvelope;

fn make_env(uid: u32, subject: &str) -> MessageEnvelope {
    MessageEnvelope {
        uid,
        subject: subject.to_string(),
        sender: "test@example.com".to_string(),
        date: "2026-10-15T10:00:00Z".to_string(),
        cc: None,
        has_attachments: false,
        is_read: true,
        is_flagged: false,
        crypto_meta: None,
    }
}

#[test]
fn test_merge_search_results_agreement_boost_and_deduplication() {
    // Local encontrou UIDs 10 (rank 0) e 20 (rank 1)
    let local = vec![make_env(10, "Doc 10"), make_env(20, "Doc 20")];

    // Remoto encontrou UIDs 30 (rank 0, histórico) e 10 (rank 1)
    let remote = vec![make_env(30, "Doc 30"), make_env(10, "Doc 10")];

    let merged = merge_search_results(&local, &remote);

    // UID 10 apareceu em ambos (acordo mútuo): deve ser o #1 absoluto
    assert_eq!(
        merged[0].uid, 10,
        "UID 10 com confirmação mútua deve liderar"
    );

    // Total de itens deve ser deduplicado (3 únicos: 10, 30, 20)
    assert_eq!(merged.len(), 3);
    let uids: Vec<u32> = merged.iter().map(|e| e.uid).collect();
    assert_eq!(uids, vec![10, 30, 20]);
}

#[test]
fn test_mutation_detection_on_empty_local_or_remote() {
    let local = vec![make_env(1, "A"), make_env(2, "B")];
    let remote: Vec<MessageEnvelope> = vec![];

    let merged_local_only = merge_search_results(&local, &remote);
    assert_eq!(merged_local_only.len(), 2);
    assert_eq!(merged_local_only[0].uid, 1);

    let merged_remote_only = merge_search_results(&[], &local);
    assert_eq!(merged_remote_only.len(), 2);
    assert_eq!(merged_remote_only[0].uid, 1);
}
