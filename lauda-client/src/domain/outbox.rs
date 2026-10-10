use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use lauda_types::ComposeForm;

/// Status de um item na fila da Outbox.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutboxStatus {
    Pending,
    Sending,
    Failed(String),
}

/// Item persistido na fila da Outbox.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxItem {
    pub id: String,
    pub form: ComposeForm,
    pub created_at_unix: u64,
    pub retry_count: u32,
    pub status: OutboxStatus,
}

/// Gerenciador de persistência da fila Outbox.
#[derive(Clone, Debug)]
pub struct OutboxQueue {
    queue_path: PathBuf,
}

impl OutboxQueue {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            queue_path: base_dir.as_ref().join("outbox").join("queue.json"),
        }
    }

    pub fn queue_path(&self) -> &Path {
        &self.queue_path
    }

    /// Retorna todos os itens da fila ordenados por data de criação.
    pub fn list(&self) -> std::io::Result<Vec<OutboxItem>> {
        unimplemented!()
    }

    /// Adiciona uma nova mensagem à fila com status Pending.
    pub fn enqueue(&self, _form: ComposeForm, _now_unix: u64) -> std::io::Result<OutboxItem> {
        unimplemented!()
    }

    /// Remove um item da fila após envio com sucesso.
    pub fn remove(&self, _id: &str) -> std::io::Result<bool> {
        unimplemented!()
    }

    /// Atualiza o status e contador de retentativas de um item.
    pub fn update_status(&self, _id: &str, _status: OutboxStatus, _increment_retry: bool) -> std::io::Result<bool> {
        unimplemented!()
    }

    /// Retorna o número de mensagens pendentes ou falhadas na fila.
    pub fn pending_count(&self) -> std::io::Result<usize> {
        unimplemented!()
    }
}
