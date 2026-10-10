use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Tipo de mutação realizada pelo usuário sobre uma mensagem.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    MarkRead { read: bool },
    MarkFlagged { flagged: bool },
    Move { dest_box: String },
    Delete,
}

/// Registro persistente de ação enfileirada.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedAction {
    pub id: String,
    pub uid: u32,
    pub mailbox: String,
    pub action: ActionType,
    pub created_at_unix: u64,
}

/// Gerenciador de persistência para reconciliação de ações locais.
#[derive(Clone, Debug)]
pub struct ActionsQueue {
    queue_path: PathBuf,
}

impl ActionsQueue {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            queue_path: base_dir.as_ref().join("sync").join("actions.json"),
        }
    }

    pub fn queue_path(&self) -> &Path {
        &self.queue_path
    }

    /// Retorna todas as ações pendentes na fila por ordem cronológica.
    pub fn list(&self) -> std::io::Result<Vec<QueuedAction>> {
        unimplemented!()
    }

    /// Enfileira uma nova ação atômica.
    pub fn enqueue(&self, _uid: u32, _mailbox: &str, _action: ActionType, _now_unix: u64) -> std::io::Result<QueuedAction> {
        unimplemented!()
    }

    /// Remove uma ação da fila após sincronização bem-sucedida.
    pub fn remove(&self, _id: &str) -> std::io::Result<bool> {
        unimplemented!()
    }

    /// Retorna a quantidade de ações pendentes de sincronização.
    pub fn count(&self) -> std::io::Result<usize> {
        unimplemented!()
    }
}
