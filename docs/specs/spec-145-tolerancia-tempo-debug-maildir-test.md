# Especificação Técnica #145 — Tolerância de Tempo em Debug no `maildir_test`

## 1. Contexto e Motivação
Na CI do Forgejo (run #70, container `gost-ci-wasm:latest`), os testes `test_maildir_large_volume_perf` e `test_maildir_search_does_not_perform_blocking_reconcile` em `lauda-server/tests/maildir_test.rs` falharam com:
- `Busca não deve bloquear síncrona com reconciliação: demorou 1.96910327s`

Ambos os testes impunham um limite estrito de tempo real (`< 500ms`) mesmo em builds de `debug` (`cargo test --workspace` sem `--release`) sob contenção de CPU/I/O do contêiner da CI.

---

## 2. Árvore de Decisão e Alternativas Descartadas (Para a Natasha)
- **Alternativa Descartada 1: Remover completamente a verificação de tempo dos dois testes:**
  - *Motivo do descarte:* Um teto razoável ainda é útil para detectar regressões graves (como travamento síncrono de 20–30s ou indexação inline de milhares de mensagens).
- **Alternativa Descartada 2: Marcar os testes com `#[ignore]`:**
  - *Motivo do descarte:* Desativaria a validação automática na CI.
- **Escolha Adotada (`maildir_perf_budget()`):**
  - Extrair um helper `maildir_perf_budget() -> Duration` em `maildir_test.rs`:
    - Em `--release` (`!cfg!(debug_assertions)`): mantém `500ms`.
    - Em `debug` (`cfg!(debug_assertions)`): usa `5s`, absorvendo contenção de I/O em containers sem mascarar travamentos reais, enquanto todas as asserções estruturais (`messages.len() == 50`, `res.sync_status.indexed_count == 0`, `res.messages.is_empty()`) permanecem estritas.

---

## 3. Critérios de Aceitação
1. `test_maildir_large_volume_perf` e `test_maildir_search_does_not_perform_blocking_reconcile` utilizam `maildir_perf_budget()`.
2. Em modo debug, `maildir_perf_budget()` retorna `Duration::from_secs(5)`.
3. Todas as asserções estruturais em `maildir_test.rs` permanecem intactas e passando.

---

## 4. Laudo de Fechamento e Homologação
*(A preencher na Fase 8 - Quality Gate)*
