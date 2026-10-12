# Especificação Técnica de Contratos: Motor SQLite de Contatos & Sync Incremental

> **Documento de Especificação e Contrato Preliminar (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#31 - feat(contacts): motor SQLite de contatos e sync incremental no lauda-server](https://codeberg.org/lumenpink/lauda/issues/31)  
> **Norma Base:** GOST 1.4.0 / RFC 6350 (vCard 4.0) / RFC 6578 (WebDAV Sync-Collection)

---

## 1. Visão Geral da Arquitetura

O `lauda-server` passa a integrar um motor de persistência SQLite embutido com journaling em modo **WAL (Write-Ahead Logging)**, otimizado para leituras concorrentes em sub-milissegundos e suporte a catálogos volumosos (20.000+ contatos) sem travamentos.

O motor atua como fonte da verdade unificada (Single Source of Truth) para:
1. Contatos autodescobertos no Maildir e e-mails de entrada/saída.
2. Contatos gerenciados e editados pelo usuário via UI do Lauda.
3. Sincronização incremental bidirecional com celulares/tablets via CardDAV (Ticket #32) e Lauda Client (Ticket #33).

---

## 2. Esquema Relacional SQLite (`contacts.db`)

O arquivo do banco será gerenciado por usuário ou compartilhado no diretório de dados do daemon (`~/.local/share/lauda/contacts.db` ou subdiretório do usuário):

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS contacts (
    id TEXT PRIMARY KEY,                       -- UUID v4 canônico
    username TEXT NOT NULL,                    -- Dono da agenda
    display_name TEXT,                         -- Nome completo de exibição
    primary_email TEXT NOT NULL,               -- E-mail normalizado em minúsculas
    secondary_emails TEXT NOT NULL DEFAULT '[]',-- JSON Array de e-mails alternativos
    phone_numbers TEXT NOT NULL DEFAULT '[]',  -- JSON Array de telefones formatados
    notes TEXT,                                -- Anotações livres
    vcard_raw TEXT NOT NULL,                   -- Payload vCard 4.0 (RFC 6350)
    has_autocrypt INTEGER NOT NULL DEFAULT 0,  -- Booleano (1 se possui chave Autocrypt ativa)
    autocrypt_fingerprint TEXT,                -- Fingerprint da chave pública
    etag TEXT NOT NULL,                        -- Hash de integridade MD5/SHA256 do vCard
    version INTEGER NOT NULL DEFAULT 1,        -- Versão sequencial de modificação
    created_at INTEGER NOT NULL,               -- Timestamp Unix Epoch (segundos)
    updated_at INTEGER NOT NULL,               -- Timestamp Unix Epoch (segundos)
    deleted_at INTEGER                         -- Tombstone para sync incremental (soft-delete)
);

CREATE INDEX IF NOT EXISTS idx_contacts_user_email 
    ON contacts(username, primary_email);

CREATE INDEX IF NOT EXISTS idx_contacts_user_updated 
    ON contacts(username, updated_at);

CREATE INDEX IF NOT EXISTS idx_contacts_user_deleted 
    ON contacts(username, deleted_at);
```

---

## 3. Contratos de Dados (`lauda-types`)

### 3.1 `ContactEntry` (Modelo Completo de Domínio)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactEntry {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub primary_email: String,
    pub secondary_emails: Vec<String>,
    pub phone_numbers: Vec<String>,
    pub notes: Option<String>,
    pub vcard_raw: String,
    pub has_autocrypt: bool,
    pub autocrypt_fingerprint: Option<String>,
    pub etag: String,
    pub version: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub deleted_at: Option<u64>,
}
```

### 3.2 `ContactSyncRequest` e `ContactSyncResponse` (Sync Incremental)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactSyncRequest {
    pub sync_token: Option<String>, // Ex: "ts:1728740000" ou ETag base
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactSyncResponse {
    pub sync_token: String,
    pub updated: Vec<ContactEntry>,
    pub deleted_ids: Vec<String>,
    pub has_more: bool,
}
```

---

## 4. Endpoints HTTP do `lauda-server`

1. **`GET /contacts`**
   - Retorna a listagem unificada paginada/ordenada de contatos ativos (`deleted_at IS NULL`).
2. **`GET /contacts/sync`**
   - Parâmetros de consulta: `sync_token` (opcional), `limit` (opcional).
   - Devolve novos/modificados desde o token + lista de `deleted_ids` (tombstones).
3. **`POST /contacts`**
   - Criação ou atualização idempotente de um contato, gerando vCard e novo ETag.
4. **`DELETE /contacts/{id}`**
   - Soft-delete do contato (marca `deleted_at = now()`, incrementa etag/token).
