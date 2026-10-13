use lauda_types::MailboxInfo;

/// Papéis canônicos padrão de pastas IMAP para categorização e ordenação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FolderRole {
    Inbox = 0,
    Sent = 1,
    Drafts = 2,
    Archive = 3,
    Spam = 4,
    Trash = 5,
    Custom = 6,
}

impl FolderRole {
    pub fn icon(&self) -> &'static str {
        match self {
            FolderRole::Inbox => "📥",
            FolderRole::Sent => "📤",
            FolderRole::Drafts => "📝",
            FolderRole::Archive => "📦",
            FolderRole::Spam => "🚫",
            FolderRole::Trash => "🗑",
            FolderRole::Custom => "📁",
        }
    }
}

/// Nó representando uma pasta na árvore ou lista hierárquica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderNode {
    pub raw_name: String,
    pub display_name: String,
    pub role: FolderRole,
    pub depth: usize,
    pub total_messages: u32,
    pub unread_messages: u32,
}

impl FolderNode {
    pub fn has_unread(&self) -> bool {
        self.unread_messages > 0
    }

    pub fn badge_text(&self) -> String {
        if self.unread_messages > 0 {
            format!("{} / {}", self.unread_messages, self.total_messages)
        } else {
            self.total_messages.to_string()
        }
    }
}

/// Detecta o papel da pasta a partir do seu nome canônico.
pub fn detect_role(name: &str) -> FolderRole {
    let lower = name.to_lowercase();
    let parts: Vec<&str> = name.split(['/', '.']).collect();
    let last = parts.last().copied().unwrap_or(name).to_lowercase();

    if lower == "inbox" {
        FolderRole::Inbox
    } else if lower == "sent" || lower.starts_with("sent") || last == "sent" {
        FolderRole::Sent
    } else if lower == "drafts" || lower.starts_with("drafts") || last == "drafts" {
        FolderRole::Drafts
    } else if lower == "archive" || lower.starts_with("archive") || last == "archive" {
        FolderRole::Archive
    } else if lower == "junk" || lower == "spam" || last == "junk" || last == "spam" {
        FolderRole::Spam
    } else if lower == "trash"
        || lower.starts_with("trash")
        || lower.contains("deleted")
        || last == "trash"
    {
        FolderRole::Trash
    } else {
        FolderRole::Custom
    }
}

/// Constrói uma lista hierárquica ordenada de `FolderNode` a partir de `MailboxInfo`.
pub fn build_folder_tree(mailboxes: &[MailboxInfo]) -> Vec<FolderNode> {
    let mut nodes: Vec<FolderNode> = mailboxes
        .iter()
        .map(|mb| {
            let role = detect_role(&mb.name);
            let parts: Vec<&str> = mb.name.split(['/', '.']).collect();
            let depth = if parts.len() > 1 { parts.len() - 1 } else { 0 };
            let display_name = parts.last().unwrap_or(&mb.name.as_str()).to_string();

            FolderNode {
                raw_name: mb.name.clone(),
                display_name,
                role,
                depth,
                total_messages: mb.total_messages,
                unread_messages: mb.unread_messages,
            }
        })
        .collect();

    // Ordenação canônica:
    // 1. Papel canônico (Inbox -> Sent -> Drafts -> Archive -> Spam -> Trash -> Custom)
    // 2. Para pastas Custom, hierarquia e ordem alfabética de caminho
    nodes.sort_by(|a, b| {
        let role_cmp = a.role.cmp(&b.role);
        if role_cmp != std::cmp::Ordering::Equal {
            role_cmp
        } else {
            a.raw_name.cmp(&b.raw_name)
        }
    });

    nodes
}
