# Especificação Técnica: Motor de Regras ManageSieve com Editor Duplo e Gerador Preditivo

- **Ticket:** #28 (`feat(rules): motor de regras ManageSieve com editor duplo e gerador preditivo`)
- **Fase DD-TDD:** Fase 3 (Spec)
- **Status:** Aprovado para implementação

---

## 1. Visão Geral e Arquitetura

O ecossistema Lauda necessita de filtragem de e-mails executada no servidor via protocolo ManageSieve (RFC 5804) e scripts Sieve (RFC 5228), garantindo que mensagens recebidas sejam organizadas automaticamente mesmo quando os clientes estiverem offline.

Esta especificação define:
1. **Modelo Estruturado de Regras Sieve (`SieveRule`):**
   - Representação intermediária e tipada para condições (`Header`, `Address`, `Size`) e ações (`FileInto`, `Discard`, `Keep`, `MarkRead`, `Stop`).
   - Parser bidirecional: gera código Sieve compatível com Dovecot Pigeonhole a partir de estruturas Rust e analisa scripts Sieve estruturados para renderização visual.
2. **Editor Duplo no Cliente:**
   - **Modo Visual (Blocos Trash Polka):** Configuração intuitiva "SE [condição] ENTÃO [ação]".
   - **Modo Hacker (Texto Puro RFC 5228):** Edição direta do código Sieve com validação de sintaxe.
3. **Gerador Preditivo de Regras ("⚡ Criar regra a partir desta mensagem"):**
   - Disponível na barra de ações da visualização de mensagem (`MessageView`).
   - Inspeciona o envelope e cabeçalhos da mensagem ativa:
     - Detecta remetente (`From`) ou domínio.
     - Detecta cabeçalhos de mailing list (`List-ID`, `List-Unsubscribe`).
     - Detecta prefixos de assunto recorrentes (e.g., `[dev]`).
     - Infere e pré-configura regra sugerindo pasta destino apropriada (e.g., `Listas/dev` ou `Contatos/Remetente`).
4. **Backend e Comunicação ManageSieve:**
   - Cliente ManageSieve assíncrono (conecta no serviço Dovecot na porta 4190 via TCP).
   - Endpoints REST no `lauda-server`:
     - `GET /rules`: lista regras parseadas e o script Sieve raw ativo.
     - `PUT /rules`: compila/valida e salva o script Sieve ativo no ManageSieve.

---

## 2. Tipos de Dados (`lauda-types` / `sieve.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionField {
    From,
    To,
    Subject,
    ListId,
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchType {
    Contains,
    Is,
    Matches, // wildcard * ?
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SieveCondition {
    pub field: ConditionField,
    pub match_type: MatchType,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SieveAction {
    FileInto(String),
    MarkRead,
    Discard,
    Keep,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SieveRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub match_all: bool, // allof vs anyof
    pub conditions: Vec<SieveCondition>,
    pub actions: Vec<SieveAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SieveScriptBundle {
    pub raw_script: String,
    pub rules: Vec<SieveRule>,
}
```

---

## 3. Gerador e Parser Sieve (RFC 5228)

- Gera cabeçalho padrão com extensões necessárias:
  ```sieve
  require ["fileinto", "imap4flags"];
  ```
- Gera blocos `if` ou `elsif`:
  ```sieve
  # rule:[Nome da Regra]
  if allof (header :contains "from" "news@example.com") {
      fileinto "Newsletters";
      stop;
  }
  ```
- O parser reconhece anotações de comentários `# rule:[Nome]` geradas pelo próprio editor para mapear perfeitamente os blocos visuais.

---

## 4. Gerador Preditivo de Regras

Assinatura no domínio:
```rust
pub fn predict_rule_from_envelope(env: &MessageEnvelope) -> SieveRule
```
- Se `env.subject` começar com colchetes (e.g. `[rust-dev]`), infere pasta `Listas/rust-dev`.
- Se o remetente for conhecido, sugere mover para pasta correspondente ou marcar estrela/lido.

---

## 5. Critérios de Aceitação & Testes

1. `test_sieve_generate_and_parse_roundtrip`: script gerado a partir de `Vec<SieveRule>` deve ser re-parseado preservando regras, condições e ações.
2. `test_predict_rule_from_envelope`: infere corretamente regras a partir de remetente e assunto.
3. `test_managesieve_protocol`: simula/valida cliente ManageSieve (banner, comandos `PUTSCRIPT`, `SETACTIVE`, `GETSCRIPT`).
4. Quality Gate completo sem regressões (`cargo test -j 2 --workspace`, `cargo clippy`, `cargo fmt`).
