# Especificação Técnica: Cache Local Unificado de Mensagens Offline (lauda-client)

- **Issue:** #16 (`feat(cache): implementar cache local de mensagens no dispositivo para acesso offline`)
- **Status:** Especificado (Fase 3 DD-TDD)
- **Data:** 2026-10-12
- **Norma:** GOST 1.4.0 / Offline-First Cache Pattern

---

## 1. Contexto e Motivação
Atualmente, no ambiente nativo (Desktop e Android), o `lauda-client` armazena mensagens em micro-arquivos JSON isolados (`mailboxes.json`, `envelopes.json`, `<uid>.json`), e no ambiente Web (WASM) utilizava `localStorage`, limitado rigidamente a 5MB.

Para garantir performance instantânea (0ms), retenção de 90 dias, navegação offline completa e sincronização contínua:
1. **Desktop & Android (Native):** Unificar a persistência em banco SQLite de alta velocidade (`messages.db` no diretório de cache/dados do app), com modo WAL, PRAGMA synchronous NORMAL e escrita transacional segura.
2. **Web (WASM):** Unificar a persistência no **IndexedDB** do navegador através da crate `rexie`, superando a barreira de cota de 5MB.
3. **Estratégia SWR (Stale-While-Revalidate):** O smolclient renderiza envelopes e mensagens do cache local de imediato e dispara background fetch para revalidação.
4. **Política de Retenção de 90 Dias:** Função atômica `purge_older_than` que remove mensagens e envelopes cujas datas ISO-8601 excedam 90 dias de antiguidade.

---

## 2. Modelagem do Banco SQLite Local (`messages.db`)

### 2.1 Tabela `cache_mailboxes`
```sql
CREATE TABLE IF NOT EXISTS cache_mailboxes (
    name TEXT PRIMARY KEY,
    total_messages INTEGER NOT NULL,
    unread_messages INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

### 2.2 Tabela `cache_envelopes`
```sql
CREATE TABLE IF NOT EXISTS cache_envelopes (
    mailbox TEXT NOT NULL,
    uid INTEGER NOT NULL,
    subject TEXT NOT NULL,
    sender TEXT NOT NULL,
    date TEXT NOT NULL,
    date_unix INTEGER NOT NULL,
    is_read INTEGER NOT NULL,
    is_flagged INTEGER NOT NULL,
    cc TEXT,
    has_attachments INTEGER NOT NULL,
    crypto_meta_json TEXT,
    PRIMARY KEY (mailbox, uid)
);

CREATE INDEX IF NOT EXISTS idx_envelopes_box_date ON cache_envelopes (mailbox, date_unix DESC);
CREATE INDEX IF NOT EXISTS idx_envelopes_box_uid ON cache_envelopes (mailbox, uid DESC);
```

### 2.3 Tabela `cache_messages`
```sql
CREATE TABLE IF NOT EXISTS cache_messages (
    mailbox TEXT NOT NULL,
    uid INTEGER NOT NULL,
    envelope_json TEXT NOT NULL,
    txt_auto TEXT,
    txt_original TEXT,
    html_sanitized TEXT,
    attachments_json TEXT NOT NULL,
    date_unix INTEGER NOT NULL,
    cached_at INTEGER NOT NULL,
    PRIMARY KEY (mailbox, uid)
);

CREATE INDEX IF NOT EXISTS idx_messages_date ON cache_messages (date_unix);
```

---

## 3. Contratos de API em `LocalMessageCache`

```rust
impl LocalMessageCache {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self;
    pub fn open<P: AsRef<Path>>(base_dir: P) -> Result<Self, std::io::Error>;
    pub fn base_dir(&self) -> &Path;

    // Mailboxes
    pub fn save_mailboxes(&self, mailboxes: &[MailboxInfo]) -> std::io::Result<()>;
    pub fn load_mailboxes(&self) -> std::io::Result<Option<Vec<MailboxInfo>>>;

    // Envelopes
    pub fn save_envelopes(&self, mailbox: &str, envelopes: &[MessageEnvelope]) -> std::io::Result<()>;
    pub fn load_envelopes(&self, mailbox: &str) -> std::io::Result<Option<Vec<MessageEnvelope>>>;

    // Mensagens Completas
    pub fn save_message(&self, mailbox: &str, msg: &Message) -> std::io::Result<()>;
    pub fn load_message(&self, mailbox: &str, uid: u32) -> std::io::Result<Option<Message>>;

    // Retenção e Purga
    pub fn purge_older_than(&self, max_age: Duration) -> std::io::Result<usize>;
}
```

---

## 4. Critérios de Aceite (Quality Gate)
1. `LocalMessageCache::save_mailboxes` e `load_mailboxes` gravam e recuperam com fidelidade.
2. `LocalMessageCache::save_envelopes` e `load_envelopes` gravam envelopes e metadados criptográficos (`crypto_meta`).
3. `LocalMessageCache::save_message` e `load_message` gravam corpo completo em texto, markdown e html sanitizado.
4. `purge_older_than` remove atomicamente registros com mais de 90 dias sem corromper mensagens recentes.
5. Todos os testes existentes (`cache_test.rs`, `swr_test.rs`, `sync_test.rs`) continuam 100% verdes.
6. `cargo clippy --workspace --all-targets -- -D warnings` sem avisos.
