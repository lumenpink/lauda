# Especificação Técnica de Contratos: UI de Contatos e Autocomplete (lauda-client)

> **Documento de Especificação e Contrato Preliminar (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#34 - feat(contacts): interface de contatos e autocomplete instantâneo no compositor](https://codeberg.org/lumenpink/lauda/issues/34)  
> **Norma Base:** GOST 1.4.0 / Optimistic UI Pattern / Neo-Brutalist Design System

---

## 1. Visão Geral da Funcionalidade

A interface do `lauda-client` integra a gestão de contatos e o autocomplete instantâneo ao fluxo de escrita:
1. **Compositor (`compose_view.rs`):** Ao digitar no campo `PARA:` ou `CC:`, sugestões baseadas no banco local (`LocalContactsCache`) aparecem em $< 5\text{ms}$, destacando chaves Autocrypt/PGP.
2. **Painel de Contatos (`contacts_view.rs`):** Carregamento imediato (0ms) via cache local, formulário para criação/edição rápida e botão para redação direta.

---

## 2. Contratos do Autocomplete (`compose_view.rs`)

### 2.1 Consulta e Sugestões
- Quando o valor digitado tiver $\ge 1$ caracteres não vazios:
  - Invoca `LocalContactsCache::search_contacts(&query, 5)`.
  - Se o retorno contiver resultados, o dropdown de sugestões é renderizado.
- O dropdown exibe:
  - **Nome Completo** (se presente) e **E-mail**.
  - **Badge PGP:** Se `has_autocrypt == true`, exibe `[🔒 PGP]`.

### 2.2 Seleção e Preenchimento
- Ao clicar em uma sugestão:
  - Preenche o input correspondente com:
    - `Nome <email>` caso haja `display_name`.
    - `email` caso não haja nome.
  - Limpa a lista de sugestões ativas imediatamente.

---

## 3. Contratos da Gestão de Contatos (`contacts_view.rs`)

### 3.1 Carregamento Otimista
- Inicializa lendo contatos diretamente de `LocalContactsCache::load_contacts()`.
- Dispara reconciliação em segundo plano contra `MailApiClient::sync_contacts`.

### 3.2 Formulário de Criação
- Campos:
  - `Nome:` (opcional)
  - `E-mail:` (obrigatório)
  - `Notas:` (opcional)
- Ao submeter:
  - Gera `ContactEntry` com UUID v4 e vCard 4.0 RFC 6350 válido.
  - Persiste via `LocalContactsCache::apply_sync_delta`.
  - Atualiza o signal reativo da UI sem recarregar a página.

### 3.3 Ações na Linha do Contato
- `✉ ESCREVER`: Redireciona para `Screen::Compose` preenchendo o destinatário.
- `🗑 EXCLUIR`: Remove do `LocalContactsCache` e atualiza a visualização.
