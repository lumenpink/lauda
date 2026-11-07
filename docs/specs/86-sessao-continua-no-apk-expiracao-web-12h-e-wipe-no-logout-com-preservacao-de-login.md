# Especificação Técnica: Detecção de Desconexão/401, Sessão Contínua no APK, Expiração Web 12h e Wipe no Logout com Preservação de Login

**ID do Ticket:** #86  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Em Desenvolvimento (Fase 3 - Spec)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Atualmente, o cliente `lauda` não trata adequadamente quando o token expira ou se invalida:
1. Erros `401 Unauthorized` (`ClientError::AuthFailed`) são capturados indistintamente como falhas genéricas de conexão, acionando apenas `is_online.set(false)`. A interface exibe a mensagem de "offline" e congela sem permitir interações e sem deslogar a sessão, forçando a usuária a adivinhar a expiração e deslogar manualmente.
2. A duração padrão da autenticação do token no servidor estava fixada em apenas 72 horas para todos os clientes, gerando expiração prematura no aplicativo móvel (APK).
3. Ao realizar logout (manual ou forçado), a rotina apagava completamente a configuração local, exigindo que a usuária digitasse novamente a URL do servidor e o usuário a cada reconexão.

---

## 2. Requisitos e Contratos

### 2.1. Duração e Auto-Emissão do Token no Servidor (`lauda-server`)
- No endpoint de login `POST /auth/login`:
  - Permitir informar `client_type` no `LoginRequest` (`"web"` ou `"app"` / `"native"`).
  - Se for `"web"`: emitir JWT com validade estrita de **12 horas**.
  - Se for `"app"` ou omitido por cliente nativo: emitir JWT com validade estendida de **1 ano** (8760 horas), garantindo experiência contínua como no Gmail/K-9.

### 2.2. Detecção Ativa de Não-Autenticado e Auto-Logout no Cliente (`lauda-client`)
- Em `lauda-client/src/ui/mod.rs` (no loop periódico de sincronização SWR) e nas operações de interface:
  - Se qualquer chamada retornar `ClientError::AuthFailed`:
    - Disparar transição imediata para `Screen::Login`.
    - Exibir banner explicativo: *"Sessão expirada. Digite sua senha para reconectar."*
    - Interromper o loop autenticado.

### 2.3. Wipe Completo no Logout com Preservação de Servidor e Usuário em Memória
- Em `lauda-client/src/domain/session.rs`:
  - Criar função canônica `logout_and_wipe(path, storage)`.
  - Passos atômicos:
    1. Reter em memória volátil `server_url` e `username`.
    2. Realizar o wipe total:
       - No navegador Web: limpar `localStorage` e caches.
       - No APK / Desktop: remover banco SQLite (`cache.db`), fila offline (`outbox.db`), índices de busca e arquivo de sessão anterior.
    3. Gravar novo arquivo/registro de sessão com `{ server_url, username, token: None }`.
    4. Garantir que `LoginView` inicialize os campos de "Servidor" e "Usuário" com esses valores mantidos.

---

## 3. Critérios de Aceite e Verificação

1. **Fase 4 (Red Test):**
   - Teste unitário e de integração em `lauda-client` verificando que o fluxo de wipe limpa o armazenamento mantendo apenas `server_url` e `username`.
   - Teste no `lauda-server` validando emissão de token com 12h para `client_type: "web"` e 8760h para `client_type: "app"`.
2. **Fase 5 (Green Implement):**
   - Implementar nos adapters do servidor e cliente.
   - Atualizar `ui/mod.rs` e `ui/inbox_view.rs` com tratamento de `ClientError::AuthFailed`.
3. **Fase 8 (Quality Gate):**
   - `cargo fmt --check` 100% aprovado.
   - `cargo clippy --all-targets -- -D warnings` sem alertas.
   - Suíte de testes aprovada.
