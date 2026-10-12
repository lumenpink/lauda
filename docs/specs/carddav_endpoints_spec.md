# Especificação Técnica de Contratos: Endpoints CardDAV Nativos

> **Documento de Especificação e Contrato Preliminar (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#32 - feat(carddav): endpoints nativos CardDAV no lauda-server](https://codeberg.org/lumenpink/lauda/issues/32)  
> **Norma Base:** GOST 1.4.0 / RFC 4791 / RFC 4918 (WebDAV) / RFC 6352 (CardDAV) / RFC 6578 (Sync-Collection)

---

## 1. Visão Geral da Arquitetura

O `lauda-server` expõe uma interface CardDAV compatível com clientes móveis nativos (Android DAVx⁵, iOS/macOS Contacts, Thunderbird). O serviço atua diretamente sobre o repositório SQLite implementado no Ticket #31, traduzindo métodos WebDAV (`PROPFIND`, `REPORT`, `PUT`, `DELETE`, `GET`) para operações atômicas no banco.

---

## 2. Rotas e Métodos HTTP

### 2.1 Descoberta Inicial
- **`GET /.well-known/carddav`**:
  - Responde com redirecionamento `301 Moved Permanently` apontando para `/dav/addressbooks/`.

### 2.2 Principals e Home Set (`PROPFIND`)
- **`PROPFIND /dav/addressbooks/`**:
  - Headers suportados: `Depth: 0` ou `Depth: 1`.
  - XML Multistatus (RFC 4918) devolvendo:
    - `current-user-principal`: `/dav/principals/{username}/`
    - `addressbook-home-set`: `/dav/addressbooks/{username}/`

- **`PROPFIND /dav/addressbooks/{username}/`**:
  - Devolve informações sobre a agenda padrão (ex: `default`):
    - `resourcetype`: `<resourcetype><collection/><addressbook xmlns="urn:ietf:params:xml:ns:carddav"/></resourcetype>`
    - `displayname`: Nome amigável (ex: `Contatos Lauda`)
    - `sync-token`: Token de sincronização atual (ex: `ts:1728700000`)

### 2.3 Consultas e Sync-Collection (`REPORT`)
- **`REPORT /dav/addressbooks/{username}/default/`**:
  - Suporta dois corpos XML principais:
    1. **`addressbook-query` / `addressbook-multiget`**:
       - Retorna os contatos correspondentes com seus respectivos payloads `vcard_raw` e `getetag`.
    2. **`sync-collection` (RFC 6578)**:
       - Extrai `<sync-token>` enviado pelo cliente.
       - Consulta o `SqliteContactsStore::sync_contacts` gerado na Issue #31.
       - Gera XML multistatus com:
         - Elementos `<response>` para contatos criados/alterados com seus respectivos vCards.
         - Elementos `<response>` com status `HTTP/1.1 404 Not Found` para contatos removidos (tombstones).
         - Elemento `<sync-token>` atualizado ao final.

### 2.4 Manipulação de vCards (`GET`, `PUT`, `DELETE`)
- **`GET /dav/addressbooks/{username}/default/{id}.vcf`**:
  - Retorna o payload cru do vCard (Content-Type: `text/vcard; charset=utf-8`) com header `ETag`.
- **`PUT /dav/addressbooks/{username}/default/{id}.vcf`**:
  - Recebe o vCard do celular/cliente, calcula ETag, parseia FN/EMAIL e persiste no SQLite via `upsert_contact`.
  - Retorna `201 Created` ou `204 No Content` com o novo `ETag`.
- **`DELETE /dav/addressbooks/{username}/default/{id}.vcf`**:
  - Executa soft-delete no SQLite (`deleted_at = now()`), gerando tombstone para os demais dispositivos.
  - Retorna `204 No Content`.

---

## 3. Autenticação e Segurança
- Autenticação via **HTTP Basic Auth** (`Authorization: Basic base64(user:pass)`).
- As credenciais são validadas de forma hermética usando a mesma infraestrutura de autenticação do `MailBackend` (`authenticate(username, password)`).
