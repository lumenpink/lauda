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

    /// Escreve atomicamente no arquivo da fila de ações.
    fn write_atomic(dest: &Path, content: &[u8]) -> std::io::Result<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let tmp_file = dest.with_extension(format!(
            "tmp.{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));

        std::fs::write(&tmp_file, content)?;
        std::fs::rename(&tmp_file, dest)?;
        Ok(())
    }

    /// Retorna todas as ações pendentes na fila por ordem cronológica.
    pub fn list(&self) -> std::io::Result<Vec<QueuedAction>> {
        if !self.queue_path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read(&self.queue_path)?;
        if content.is_empty() {
            return Ok(Vec::new());
        }
        let mut actions: Vec<QueuedAction> = serde_json::from_slice(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        actions.sort_by_key(|a| a.created_at_unix);
        Ok(actions)
    }

    /// Salva toda a coleção de ações atomicamente.
    fn save_all(&self, actions: &[QueuedAction]) -> std::io::Result<()> {
        let json = serde_json::to_vec_pretty(actions)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Self::write_atomic(&self.queue_path, &json)
    }

    /// Enfileira uma nova ação atômica.
    pub fn enqueue(&self, uid: u32, mailbox: &str, action: ActionType, now_unix: u64) -> std::io::Result<QueuedAction> {
        let mut actions = self.list()?;
        let id = format!(
            "act_{}_{}_{}",
            uid,
            now_unix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() % 1_000_000
        );

        let queued = QueuedAction {
            id,
            uid,
            mailbox: mailbox.to_string(),
            action,
            created_at_unix: now_unix,
        };

        actions.push(queued.clone());
        self.save_all(&actions)?;
        Ok(queued)
    }

    /// Remove uma ação da fila após sincronização bem-sucedida.
    pub fn remove(&self, id: &str) -> std::io::Result<bool> {
        let mut actions = self.list()?;
        let initial_len = actions.len();
        actions.retain(|a| a.id != id);
        if actions.len() != initial_len {
            self.save_all(&actions)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Retorna a quantidade de ações pendentes de sincronização.
    pub fn count(&self) -> std::io::Result<usize> {
        let actions = self.list()?;
        Ok(actions.len())
    }
}
