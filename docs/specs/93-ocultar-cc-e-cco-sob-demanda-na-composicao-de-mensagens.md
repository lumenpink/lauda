# Especificação Técnica — Ticket #93
## Ocultar CC e CCO Sob Demanda na Composição de Mensagens

- **Ticket:** #93
- **Branch:** `feat/93-ocultar-cc-e-cco-sob-demanda-na-composicao-de-mensagens`
- **Data:** 2026-11-09

---

### 1. Contexto e Motivação
Na tela de composição de mensagens (`ComposeView`), o campo "CC (OPCIONAL)" ficava sempre visível entre os campos "PARA" e "ASSUNTO".
1. A navegação sequencial via teclado (`Tab`) parava obrigatoriamente no campo CC, gerando fricção no fluxo de redação rápida para a esmagadora maioria dos e-mails comuns.
2. O compositor não possuía suporte a Cópia Oculta (CCO / BCC).
3. Ao responder a mensagens que já possuem múltiplos destinatários em cópia, os campos devem estar imediatamente visíveis para conferência e edição.

---

### 2. Requisitos Técnicos

#### 2.1 Modelo de Dados (`lauda-types`)
- Adicionar o campo `pub bcc: Option<String>` à struct `ComposeForm`:
  ```rust
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bcc: Option<String>,
  ```
- Garantir compatibilidade com serialização e desserialização via `serde_json`.

#### 2.2 Interface do Usuário (`lauda-client`)
- Na tela `ComposeView`:
  - Introduzir sinal `show_cc_bcc: Signal<bool>` inicializado com `true` se `reply_cc` estiver presente e não-vazio; caso contrário, `false`.
  - Introduzir sinal `bcc_input: Signal<String>` inicializado vazio.
  - Quando `show_cc_bcc` for `false`, os campos de texto CC e CCO não são renderizados no fluxo de tabulação, e um botão de ação rápida (ex.: `+ CC / CCO`) é exibido junto ao campo de destinatário para expansão voluntária.
  - Quando `show_cc_bcc` for `true`, renderizar os campos:
    - `CC (OPCIONAL):`
    - `CCO (OPCIONAL):`
  - No envio da mensagem (`handle_send`), higienizar e incluir `bcc` no `ComposeForm` despachado para a fila `Outbox`.

#### 2.3 Backend e Despacho (`lauda-server`)
- Em `MaildirBackend::send_message`:
  - Extrair destinatários do campo `form.bcc` e incorporá-los à lista de `recipients` para entrega/despacho via SMTP/Sendmail:
    ```rust
    if let Some(ref bcc) = form.bcc {
        recipients.extend(extract_recipient_emails(bcc));
    }
    ```

---

### 3. Fases GOST DD-TDD
- **Fase 3 (Spec):** Elaboração deste documento de especificação técnica.
- **Fase 4 (Red Test):** Testes unitários em `lauda-types` e `lauda-server` validando a existência e o comportamento do campo `bcc`.
- **Fase 5 (Green Implement):** Implementação de `bcc` em `lauda-types`, expansão sob demanda em `ComposeView` (`lauda-client`) e extração de destinatários em `lauda-server`.
- **Fase 6 (Refactor & Mutation):** Testes de mutação e validação de regressões.
- **Fase 7 (Style & Changelog):** Formatação `cargo fmt`, lints `clippy` e registro no `CHANGELOG.md`.
- **Fase 8 (Quality Gate):** Verificação rigorosa com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cccp check --strict`.
- **Fase 9 (Finish):** Fechamento e merge canônico via CCCP.

---

### 4. Status
- **Status:** CONCLUÍDO (CONCLUDED)
- **Validação:** Testes de contrato aprovados em `compose_cc_bcc_contract_test`, `contracts_test` e `sendmail_isolation_test`.
