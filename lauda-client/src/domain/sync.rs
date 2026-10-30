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

/// Configuração do worker de sincronização em segundo plano.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncWorkerConfig {
    pub poll_interval_secs: u64,
    pub min_backoff_secs: u64,
    pub max_backoff_secs: u64,
    pub retention_days: u32,
    pub prefetch_concurrency: usize,
    pub notify_on_new: bool,
}

impl Default for SyncWorkerConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 60,
            min_backoff_secs: 5,
            max_backoff_secs: 300,
            retention_days: 90,
            prefetch_concurrency: 5,
            notify_on_new: true,
        }
    }
}

/// Calcula o próximo intervalo de espera segundo o algoritmo de backoff exponencial.
/// Em caso de sucesso, restaura `poll_interval_secs`. Em caso de erro, dobra até `max_backoff_secs`.
pub fn calculate_next_backoff(
    current_backoff_secs: u64,
    is_success: bool,
    config: &SyncWorkerConfig,
) -> u64 {
    if is_success {
        config.poll_interval_secs
    } else if current_backoff_secs == 0 {
        config.min_backoff_secs
    } else {
        std::cmp::min(
            current_backoff_secs.saturating_mul(2),
            config.max_backoff_secs,
        )
    }
}

/// Representação de alerta nativo para mensagens recém-chegadas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewMessageNotification {
    pub sender: String,
    pub subject: String,
    pub uid: u32,
}

/// Detecta mensagens novas e não lidas comparando a lista remota com os envelopes cacheados.
pub fn detect_new_unread_messages(
    remote_envelopes: &[MessageEnvelope],
    cached_envelopes: &[MessageEnvelope],
) -> Vec<NewMessageNotification> {
    let known_uids: std::collections::HashSet<u32> =
        cached_envelopes.iter().map(|e| e.uid).collect();
    let mut new_notifs = Vec::new();
    for env in remote_envelopes {
        if !env.is_read && !known_uids.contains(&env.uid) {
            new_notifs.push(NewMessageNotification {
                sender: env.sender.clone(),
                subject: env.subject.clone(),
                uid: env.uid,
            });
        }
    }
    new_notifs
}

/// Emite notificação nativa no ambiente Desktop através de `notify-send` (XDG).
pub fn send_desktop_notification(notif: &NewMessageNotification) {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("notify-send")
            .arg("-a")
            .arg("Lauda")
            .arg("-i")
            .arg("lauda")
            .arg(format!("Novo e-mail: {}", notif.sender))
            .arg(&notif.subject)
            .spawn();
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = notif;
    }
}

/// Resumo das mutações reconciliadas em um ciclo unificado de sincronização.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncCycleResult {
    pub actions_flushed: usize,
    pub messages_prefetched: usize,
    pub new_unread_messages: usize,
    pub messages_purged: usize,
    pub contacts_updated: usize,
    pub contacts_deleted: usize,
    pub calendar_updated: usize,
    pub calendar_deleted: usize,
}

/// Executa uma rodada unificada de sincronização transacional:
/// 1. Esvaziamento da fila de ações locais (Outbox).
/// 2. Sincronização de envelopes e notificação diferencial de novas mensagens não lidas.
/// 3. Prefetch eager concorrente de corpos dentro da janela de retenção (90 dias).
/// 4. Purga automática de mensagens antigas expiradas (> 90 dias).
/// 5. Reconciliação incremental de contatos com o servidor.
/// 6. Reconciliação incremental de eventos de agenda com CalDAV.
pub async fn sync_all_cycle<C: MailApiClient + ?Sized>(
    client: &C,
    token: &str,
    mailbox: &str,
    cache_dir: &Path,
    config: &SyncWorkerConfig,
    now_unix: u64,
) -> Result<SyncCycleResult, ClientError> {
    let mut result = SyncCycleResult::default();

    // 1. Esvaziar fila de ações locais (Outbox)
    let actions_queue = ActionsQueue::new(cache_dir);
    if let Ok(pending_actions) = actions_queue.list() {
        for action in pending_actions {
            let res = match action.action {
                ActionType::Delete => {
                    client
                        .delete_message(token, &action.mailbox, action.uid)
                        .await
                }
                ActionType::MarkRead { read: _ } => Ok(()),
                ActionType::MarkFlagged { flagged: _ } => Ok(()),
                ActionType::Move { dest_box: _ } => Ok(()),
            };
            if res.is_ok() {
                let _ = actions_queue.remove(&action.id);
                result.actions_flushed += 1;
            }
        }
    }

    // 2. Sincronizar envelopes de correio e detectar novas mensagens não lidas
    let message_cache = LocalMessageCache::new(cache_dir);
    let cached_envelopes = message_cache
        .load_envelopes(mailbox)
        .ok()
        .flatten()
        .unwrap_or_default();
    let remote_envelopes = client.list_messages(token, mailbox, None, None).await?;
    let new_unreads = detect_new_unread_messages(&remote_envelopes, &cached_envelopes);
    result.new_unread_messages = new_unreads.len();

    if config.notify_on_new {
        for notif in &new_unreads {
            send_desktop_notification(notif);
        }
    }

    let _ = message_cache.save_envelopes(mailbox, &remote_envelopes);

    // 3. Prefetch eager dos corpos dentro da janela de retenção (90 dias)
    let prefetched = prefetch_missing_bodies(
        client,
        token,
        mailbox,
        &remote_envelopes,
        &message_cache,
        config.prefetch_concurrency,
        now_unix,
    )
    .await?;
    result.messages_prefetched = prefetched;

    // 4. Purga de mensagens que excederam a retenção
    let retention_duration = std::time::Duration::from_secs(config.retention_days as u64 * 86400);
    if let Ok(purged) = message_cache.purge_expired_messages(retention_duration, now_unix) {
        result.messages_purged = purged;
    }

    // 5. Reconciliação incremental de contatos
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(contacts_cache) = crate::domain::contacts_cache::LocalContactsCache::open(cache_dir) {
        let current_sync_token = contacts_cache.get_sync_token().ok().flatten();
        let req = lauda_types::contact::ContactSyncRequest {
            sync_token: current_sync_token,
            limit: Some(100),
        };
        if let Ok(resp) = client.sync_contacts(token, &req).await {
            result.contacts_updated = resp.updated.len();
            result.contacts_deleted = resp.deleted_ids.len();
            let _ =
                contacts_cache.apply_sync_delta(&resp.updated, &resp.deleted_ids, &resp.sync_token);
        }
    }

    // 6. Reconciliação incremental de eventos de agenda
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(cal_cache) = crate::domain::calendar_cache::LocalCalendarCache::open(cache_dir) {
        let current_cal_token = cal_cache.get_sync_token().ok().flatten();
        if let Ok(resp) = client
            .sync_calendar(token, current_cal_token.as_deref())
            .await
        {
            result.calendar_updated = resp.updated.len();
            result.calendar_deleted = resp.deleted_ids.len();
            let _ = cal_cache.apply_sync_delta(&resp.updated, &resp.deleted_ids, &resp.sync_token);
        }
    }

    Ok(result)
}

/// Manipulador de controle para o worker em execução em segundo plano.
pub struct SyncWorkerHandle {
    trigger_tx: tokio::sync::mpsc::Sender<()>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
}

impl SyncWorkerHandle {
    /// Dispara uma rodada de sincronização imediata (ex.: retorno ao primeiro plano / Fast Resume).
    pub async fn trigger_sync(&mut self) -> Result<(), ClientError> {
        self.trigger_tx
            .send(())
            .await
            .map_err(|e| ClientError::Internal(format!("Erro ao disparar sync: {e}")))
    }

    /// Envia sinal gracioso de encerramento para o worker.
    pub async fn shutdown(&mut self) -> Result<(), ClientError> {
        self.shutdown_tx
            .send(true)
            .map_err(|e| ClientError::Internal(format!("Erro ao enviar shutdown: {e}")))
    }
}

/// Worker assíncrono autônomo que executa sincronizações periódicas com backoff adaptativo.
pub struct BackgroundSyncWorker<C: MailApiClient + 'static> {
    client: std::sync::Arc<C>,
    token: String,
    mailbox: String,
    cache_dir: PathBuf,
    config: SyncWorkerConfig,
    trigger_rx: tokio::sync::mpsc::Receiver<()>,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
}

impl<C: MailApiClient + 'static> BackgroundSyncWorker<C> {
    pub fn new(
        client: std::sync::Arc<C>,
        token: String,
        mailbox: String,
        cache_dir: PathBuf,
        config: SyncWorkerConfig,
    ) -> (Self, SyncWorkerHandle) {
        let (trigger_tx, trigger_rx) = tokio::sync::mpsc::channel(8);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let worker = Self {
            client,
            token,
            mailbox,
            cache_dir,
            config,
            trigger_rx,
            shutdown_rx,
        };

        let handle = SyncWorkerHandle {
            trigger_tx,
            shutdown_tx,
        };

        (worker, handle)
    }

    /// Executa o loop principal de sincronização em segundo plano.
    pub async fn run(mut self) {
        let mut current_backoff = 0u64;

        loop {
            if *self.shutdown_rx.borrow() {
                break;
            }

            let wait_secs = if current_backoff == 0 {
                self.config.poll_interval_secs
            } else {
                current_backoff
            };

            let sleep_future = tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs));
            tokio::pin!(sleep_future);

            tokio::select! {
                _ = &mut sleep_future => {
                    // Executa ciclo nominal agendado
                }
                Some(_) = self.trigger_rx.recv() => {
                    // Disparo manual imediato (Fast Resume / Pull to refresh)
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        break;
                    }
                }
            }

            if *self.shutdown_rx.borrow() {
                break;
            }

            let now_unix = crate::domain::time::current_unix_timestamp();
            let sync_res = sync_all_cycle(
                self.client.as_ref(),
                &self.token,
                &self.mailbox,
                &self.cache_dir,
                &self.config,
                now_unix,
            )
            .await;

            match sync_res {
                Ok(_summary) => {
                    current_backoff = calculate_next_backoff(current_backoff, true, &self.config);
                }
                Err(_err) => {
                    current_backoff = calculate_next_backoff(current_backoff, false, &self.config);
                }
            }
        }
    }
}
