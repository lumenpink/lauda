# Especificação Técnica de Contratos: Cache Local de Contatos (lauda-client)

> **Documento de Especificação e Contrato Preliminar (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#33 - feat(contacts): cache local de contatos no lauda-client (IndexedDB & SQLite)](https://codeberg.org/lumenpink/lauda/issues/33)  
> **Norma Base:** GOST 1.4.0 / Offline-First Cache Pattern / RFC 6350 / RFC 6578

---

## 1. Visão Geral da Arquitetura

O `lauda-client` implementa uma camada de persistência local offline-first capaz de armazenar mais de 20.000 contatos com carregamento instantâneo (0ms) e sincronização incremental contra o `lauda-server`.

A arquitetura resolve a discrepância entre ambientes:
- **Desktop & Android (Native):** Banco de dados SQLite local (`contacts.db` no diretório de cache do app) gerenciado via `rusqlite`, garantindo escrita atômica em transação e consultas de prefixo ultra-rápidas.
- **Web (WASM):** Banco de dados **IndexedDB** nativo do navegador via `rexie`, superando a limitação de cota de 5MB do `localStorage` e operando de forma assíncrona.

---

## 2. Contrato da API Remota (lauda-server)

### Endpoint `POST /contacts/sync`
- **Headers:** `Authorization: Bearer <token_jwt>`
- **Content-Type:** `application/json`
- **Request Body (`ContactSyncRequest`):**
  ```json
  {
    "sync_token": "ts:1728700000",
    "limit": 500
  }
  ```
- **Response Body (`ContactSyncResponse`):**
  ```json
  {
    "sync_token": "ts:1728700500",
    "updated": [
      {
        "id": "c1-uuid",
        "username": "lumen",
        "display_name": "Maria Firmina",
        "primary_email": "maria@letras.org",
        "secondary_emails": [],
        "phone_numbers": [],
        "notes": null,
        "vcard_raw": "BEGIN:VCARD...",
        "has_autocrypt": false,
        "autocrypt_fingerprint": null,
        "etag": "a1b2c3d4",
        "version": 1,
        "created_at": 1728700100,
        "updated_at": 1728700500,
        "deleted_at": null
      }
    ],
    "deleted_ids": ["c2-deleted-uuid"],
    "has_more": false
  }
  ```

---

## 3. Contrato da Trait `MailApiClient`

Extensão da trait no `lauda-client`:

```rust
#[async_trait]
pub trait MailApiClient: Send + Sync {
    // ... métodos existentes ...

    async fn sync_contacts(
        &self,
        token: &str,
        req: &ContactSyncRequest,
    ) -> Result<ContactSyncResponse, ClientError>;
}
```

---

## 4. Contrato do Cache Local (`LocalContactsCache`)

A abstração local de contatos define operações atômicas:

```rust
pub trait LocalContactsStore: Send + Sync {
    /// Carrega todos os contatos ativos ordenados por nome/e-mail.
    fn load_contacts(&self) -> Result<Vec<ContactEntry>, ClientError>;

    /// Busca contatos por prefixo ou substring no nome ou e-mail (para autocomplete #34).
    fn search_contacts(&self, query: &str, limit: usize) -> Result<Vec<ContactEntry>, ClientError>;

    /// Aplica em lote uma resposta incremental (upserts e tombstones) e atualiza o token.
    fn apply_sync_delta(
        &self,
        updated: &[ContactEntry],
        deleted_ids: &[String],
        new_token: &str,
    ) -> Result<(), ClientError>;

    /// Retorna o último sync_token armazenado (ou None na primeira execução).
    fn get_sync_token(&self) -> Result<Option<String>, ClientError>;

    /// Retorna a contagem total de contatos ativos no cache local.
    fn count_contacts(&self) -> Result<usize, ClientError>;
}
```

---

## 5. Estratégia de Armazenamento e Tabelas (SQLite Nativo)

No SQLite (`contacts.db`):
- **Tabela `contacts`:**
  - `id TEXT PRIMARY KEY`
  - `username TEXT NOT NULL`
  - `display_name TEXT`
  - `primary_email TEXT NOT NULL`
  - `secondary_emails TEXT NOT NULL` (JSON array)
  - `phone_numbers TEXT NOT NULL` (JSON array)
  - `notes TEXT`
  - `vcard_raw TEXT NOT NULL`
  - `has_autocrypt INTEGER NOT NULL DEFAULT 0`
  - `autocrypt_fingerprint TEXT`
  - `etag TEXT NOT NULL`
  - `version INTEGER NOT NULL DEFAULT 1`
  - `created_at INTEGER NOT NULL`
  - `updated_at INTEGER NOT NULL`
  - Índices: `idx_contacts_email` em `primary_email`, `idx_contacts_name` em `display_name`.
- **Tabela `metadata`:**
  - `key TEXT PRIMARY KEY`, `value TEXT NOT NULL`.
  - Chave `"sync_token"`.
