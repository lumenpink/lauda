use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

    /// Retorna todas as ações pendentes na fila por ordem cronológica.
    pub fn list(&self) -> std::io::Result<Vec<QueuedAction>> {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
                && let Ok(Some(content)) = storage.get_item("lauda_actions_queue")
            {
                let mut actions: Vec<QueuedAction> = serde_json::from_str(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                actions.sort_by_key(|a| a.created_at_unix);
                Ok(actions)
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
            let mut actions: Vec<QueuedAction> = serde_json::from_slice(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            actions.sort_by_key(|a| a.created_at_unix);
            Ok(actions)
        }
    }

    /// Salva toda a coleção de ações atomicamente.
    fn save_all(&self, actions: &[QueuedAction]) -> std::io::Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
                && let Ok(json) = serde_json::to_string(actions)
            {
                let _ = storage.set_item("lauda_actions_queue", &json);
            }
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let json = serde_json::to_vec_pretty(actions)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Self::write_atomic(&self.queue_path, &json)
        }
    }

    /// Enfileira uma nova ação atômica.
    pub fn enqueue(
        &self,
        uid: u32,
        mailbox: &str,
        action: ActionType,
        now_unix: u64,
    ) -> std::io::Result<QueuedAction> {
        let mut actions = self.list()?;
        let id = format!(
            "act_{}_{}_{}",
            uid,
            now_unix,
            crate::domain::time::current_unix_timestamp() % 1_000_000
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

use crate::domain::cache::LocalMessageCache;
use crate::domain::ports::{ClientError, MailApiClient};
use lauda_types::MessageEnvelope;

/// Janela normativa padrão para sincronização e retenção integral de mensagens (90 dias).
pub const RETENTION_90_DAYS_SECS: u64 = 90 * 86400;

/// Estrutura para executar um conjunto de futures em paralelo de forma cooperativa sem dependências externas.
struct BatchJoin<F: std::future::Future> {
    futures: Vec<Option<F>>,
    results: Vec<Option<F::Output>>,
    completed: usize,
}

impl<F> BatchJoin<F>
where
    F: std::future::Future + Unpin,
    F::Output: Unpin,
{
    fn new(futs: Vec<F>) -> Self {
        let len = futs.len();
        Self {
            futures: futs.into_iter().map(Some).collect(),
            results: (0..len).map(|_| None).collect(),
            completed: 0,
        }
    }
}

impl<F> std::future::Future for BatchJoin<F>
where
    F: std::future::Future + Unpin,
    F::Output: Unpin,
{
    type Output = Vec<F::Output>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        for i in 0..this.futures.len() {
            if let Some(fut) = &mut this.futures[i]
                && let std::task::Poll::Ready(val) = std::pin::Pin::new(fut).poll(cx)
            {
                this.futures[i] = None;
                this.results[i] = Some(val);
                this.completed += 1;
            }
        }

        if this.completed == this.results.len() {
            let out = std::mem::take(&mut this.results)
                .into_iter()
                .map(|o| o.unwrap())
                .collect();
            std::task::Poll::Ready(out)
        } else {
            std::task::Poll::Pending
        }
    }
}

/// Prefetch eager e concorrente dos corpos de mensagens situadas na janela de 90 dias
/// que ainda não existem no cache local.
///
/// Prioriza as mensagens mais recentes (ordem decrescente de UID).
/// Executa as requisições em lotes assíncronos controlados por `concurrency_limit` (ex.: 5).
pub async fn prefetch_missing_bodies<C: MailApiClient + ?Sized>(
    client: &C,
    token: &str,
    mailbox: &str,
    envelopes: &[MessageEnvelope],
    cache: &LocalMessageCache,
    concurrency_limit: usize,
    now_unix: u64,
) -> Result<usize, ClientError> {
    let cutoff_unix = now_unix.saturating_sub(RETENTION_90_DAYS_SECS);

    let mut missing_uids = Vec::new();
    for env in envelopes {
        let env_unix = LocalMessageCache::parse_iso_date_to_unix(&env.date).unwrap_or(0);
        if env_unix >= cutoff_unix {
            let already_cached = cache
                .load_message(mailbox, env.uid)
                .ok()
                .flatten()
                .is_some();
            if !already_cached {
                missing_uids.push(env.uid);
            }
        }
    }

    if missing_uids.is_empty() {
        return Ok(0);
    }

    // Mais recentes primeiro
    missing_uids.sort_by(|a, b| b.cmp(a));

    let max_concurrent = std::cmp::max(1, concurrency_limit);
    let mut total_prefetched = 0;

    for chunk in missing_uids.chunks(max_concurrent) {
        let mut futs = Vec::with_capacity(chunk.len());
        for &uid in chunk {
            futs.push(Box::pin(client.get_message(token, mailbox, uid)));
        }

        let results = BatchJoin::new(futs).await;
        for full_msg in results.into_iter().flatten() {
            if cache.save_message(mailbox, &full_msg).is_ok() {
                total_prefetched += 1;
            }
        }
    }

    Ok(total_prefetched)
}

/// Prefetch eager dos corpos das mensagens retornadas por busca remota que ainda não estão em cache,
/// garantindo 0 RTT na abertura imediata ao clicar.
pub async fn prefetch_search_result_bodies<C: MailApiClient + ?Sized>(
    client: &C,
    token: &str,
    mailbox: &str,
    envelopes: &[MessageEnvelope],
    cache: &LocalMessageCache,
    concurrency_limit: usize,
) -> Result<usize, ClientError> {
    let mut missing_uids = Vec::new();
    for env in envelopes {
        let already_cached = cache
            .load_message(mailbox, env.uid)
            .ok()
            .flatten()
            .is_some();
        if !already_cached {
            missing_uids.push(env.uid);
        }
    }

    if missing_uids.is_empty() {
        return Ok(0);
    }

    let max_concurrent = std::cmp::max(1, concurrency_limit);
    let mut total_prefetched = 0;

    for chunk in missing_uids.chunks(max_concurrent) {
        let mut futs = Vec::with_capacity(chunk.len());
        for &uid in chunk {
            futs.push(Box::pin(client.get_message(token, mailbox, uid)));
        }

        let results = BatchJoin::new(futs).await;
        for full_msg in results.into_iter().flatten() {
            if cache.save_message(mailbox, &full_msg).is_ok() {
                total_prefetched += 1;
            }
        }
    }

    Ok(total_prefetched)
}
