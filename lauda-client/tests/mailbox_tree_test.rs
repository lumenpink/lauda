use lauda_client::domain::folders::{FolderNode, FolderRole, build_folder_tree};
use lauda_types::MailboxInfo;

#[test]
fn test_build_folder_tree_canonical_order_and_roles() {
    let raw_mailboxes = vec![
        MailboxInfo {
            name: "Trash".to_string(),
            total_messages: 5,
            unread_messages: 0,
        },
        MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 100,
            unread_messages: 3,
        },
        MailboxInfo {
            name: "Sent".to_string(),
            total_messages: 40,
            unread_messages: 0,
        },
        MailboxInfo {
            name: "Drafts".to_string(),
            total_messages: 2,
            unread_messages: 1,
        },
        MailboxInfo {
            name: "Archive".to_string(),
            total_messages: 500,
            unread_messages: 0,
        },
        MailboxInfo {
            name: "Junk".to_string(),
            total_messages: 10,
            unread_messages: 2,
        },
    ];

    let tree = build_folder_tree(&raw_mailboxes);

    // Deve ordenar: INBOX, Sent, Drafts, Archive, Junk/Spam, Trash
    let roles: Vec<FolderRole> = tree.iter().map(|n| n.role).collect();
    assert_eq!(
        roles,
        vec![
            FolderRole::Inbox,
            FolderRole::Sent,
            FolderRole::Drafts,
            FolderRole::Archive,
            FolderRole::Spam,
            FolderRole::Trash,
        ]
    );

    // Profundidade da raiz deve ser 0
    for node in &tree {
        assert_eq!(node.depth, 0);
    }
}

#[test]
fn test_build_folder_tree_nested_slash_and_dot_hierarchy() {
    let raw_mailboxes = vec![
        MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 50,
            unread_messages: 2,
        },
        MailboxInfo {
            name: "INBOX/Projetos".to_string(),
            total_messages: 10,
            unread_messages: 0,
        },
        MailboxInfo {
            name: "INBOX/Projetos/Lauda".to_string(),
            total_messages: 8,
            unread_messages: 1,
        },
        MailboxInfo {
            name: "Trabalho.Clientes".to_string(),
            total_messages: 4,
            unread_messages: 0,
        },
    ];

    let tree = build_folder_tree(&raw_mailboxes);

    let inbox = tree.iter().find(|n| n.raw_name == "INBOX").unwrap();
    assert_eq!(inbox.depth, 0);
    assert_eq!(inbox.role, FolderRole::Inbox);

    let projetos = tree.iter().find(|n| n.raw_name == "INBOX/Projetos").unwrap();
    assert_eq!(projetos.depth, 1);
    assert_eq!(projetos.display_name, "Projetos");
    assert_eq!(projetos.role, FolderRole::Custom);

    let lauda = tree
        .iter()
        .find(|n| n.raw_name == "INBOX/Projetos/Lauda")
        .unwrap();
    assert_eq!(lauda.depth, 2);
    assert_eq!(lauda.display_name, "Lauda");
    assert_eq!(lauda.unread_messages, 1);

    let clientes = tree
        .iter()
        .find(|n| n.raw_name == "Trabalho.Clientes")
        .unwrap();
    assert_eq!(clientes.depth, 1);
    assert_eq!(clientes.display_name, "Clientes");
}

#[test]
fn test_folder_node_badge_labels() {
    let node_unread = FolderNode {
        raw_name: "INBOX".to_string(),
        display_name: "INBOX".to_string(),
        role: FolderRole::Inbox,
        depth: 0,
        total_messages: 25,
        unread_messages: 4,
    };
    assert_eq!(node_unread.badge_text(), "4 / 25");
    assert!(node_unread.has_unread());

    let node_read = FolderNode {
        raw_name: "Sent".to_string(),
        display_name: "Sent".to_string(),
        role: FolderRole::Sent,
        depth: 0,
        total_messages: 18,
        unread_messages: 0,
    };
    assert_eq!(node_read.badge_text(), "18");
    assert!(!node_read.has_unread());
}
