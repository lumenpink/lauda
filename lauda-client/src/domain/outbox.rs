use lauda_types::ComposeForm;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

    /// Escreve atomicamente no arquivo da fila.
    #[allow(dead_code)]
    fn write_atomic(dest: &Path, content: &[u8]) -> std::io::Result<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let tmp_file = dest.with_extension(format!(
            "tmp.{}",
            crate::domain::time::current_unix_timestamp()
        ));

        std::fs::write(&tmp_file, content)?;
        std::fs::rename(&tmp_file, dest)?;
        Ok(())
    }

    /// Retorna todos os itens da fila ordenados por data de criação.
    pub fn list(&self) -> std::io::Result<Vec<OutboxItem>> {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
                && let Ok(Some(content)) = storage.get_item("lauda_outbox_queue")
            {
                let mut items: Vec<OutboxItem> = serde_json::from_str(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                items.sort_by_key(|item| item.created_at_unix);
                Ok(items)
            } else {
                Ok(Vec::new())
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !self.queue_path.exists() {
                return Ok(Vec::new());
            }
            let content = std::fs::read(&self.queue_path)?;
            if content.is_empty() {
                return Ok(Vec::new());
            }
            let mut items: Vec<OutboxItem> = serde_json::from_slice(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            items.sort_by_key(|item| item.created_at_unix);
            Ok(items)
        }
    }

    /// Grava a lista completa de itens de volta para a fila.
    fn save_all(&self, items: &[OutboxItem]) -> std::io::Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
                && let Ok(json) = serde_json::to_string(items)
            {
                let _ = storage.set_item("lauda_outbox_queue", &json);
            }
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let json = serde_json::to_vec_pretty(items)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Self::write_atomic(&self.queue_path, &json)
        }
    }

    /// Adiciona uma nova mensagem à fila com status Pending.
    pub fn enqueue(&self, form: ComposeForm, now_unix: u64) -> std::io::Result<OutboxItem> {
        let mut items = self.list()?;
        let id = format!(
            "out_{}_{}",
            now_unix,
            crate::domain::time::current_unix_timestamp() % 1_000_000
        );

        let item = OutboxItem {
            id,
            form,
            created_at_unix: now_unix,
            retry_count: 0,
            status: OutboxStatus::Pending,
        };

        items.push(item.clone());
        self.save_all(&items)?;
        Ok(item)
    }

    /// Remove um item da fila após envio com sucesso.
    pub fn remove(&self, id: &str) -> std::io::Result<bool> {
        let mut items = self.list()?;
        let initial_len = items.len();
        items.retain(|item| item.id != id);
        if items.len() != initial_len {
            self.save_all(&items)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Atualiza o status e contador de retentativas de um item.
    pub fn update_status(
        &self,
        id: &str,
        status: OutboxStatus,
        increment_retry: bool,
    ) -> std::io::Result<bool> {
        let mut items = self.list()?;
        let mut found = false;
        for item in &mut items {
            if item.id == id {
                item.status = status;
                if increment_retry {
                    item.retry_count += 1;
                }
                found = true;
                break;
            }
        }

        if found {
            self.save_all(&items)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Retorna o número de mensagens pendentes ou falhadas na fila.
    pub fn pending_count(&self) -> std::io::Result<usize> {
        let items = self.list()?;
        Ok(items.len())
    }
}
