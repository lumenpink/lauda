# Especificação Técnica: Atualização Automática da Lista de E-mails Pós-Sincronização

**ID do Ticket:** #102  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Concluído (Fase 9)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Durante os testes em dispositivo móvel com a versão APK, foi constatado o seguinte comportamento:
Ao concluir a sincronização em segundo plano e exibir o status "● Sincronizado", a lista de e-mails na tela permaneceu congelada com os itens anteriores, exigindo que o usuário clicasse manualmente no botão de atualizar da interface para que as novas mensagens aparecessem.

### 1.1 Diagnóstico do Problema
1. Em `lauda-client/src/ui/mod.rs`, antes do `sync_all_cycle`, a verificação condicional exigia `messages.read().len() <= 50`. Como a sincronização unificada salva até 100 envelopes no cache local e a inicialização do app carrega o cache local completo, `messages.read().len()` frequentemente continha 100 mensagens, fazendo com que a condição avaliasse para `false`.
2. Após o término de `sync_all_cycle`, o loop em segundo plano apenas recalculava o contador de pendências (`sync_pending_count.set(remaining)`) e entrava em sono de 60 segundos (`async_sleep`). Em nenhum momento o sinal reativo `messages` da tela era atualizado com os novos envelopes reconciliados e salvos no `LocalMessageCache`.
3. Dessa forma, as mensagens existiam no SQLite local e no servidor, mas a visualização do Dioxus nunca era notificada da mudança.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: Notificação Reativa Imediata Pós-`sync_all_cycle`:**
  - *Alternativa descartada:* Aguardar clique manual ou evento do WebSocket.
    - *Motivo do descarte:* Quebra a expectativa de sincronização automática e transparente. Se o ciclo concluiu e os dados estão salvos no cache, a tela deve refletir a nova verdade de forma reativa.
  - *Escolha adotada:* Logo após o `await` do `sync_all_cycle`, verificar se a caixa em exibição (`current_mailbox.read()`) corresponde à caixa sincronizada (`active_box`). Se sim, ler os envelopes atualizados do `LocalMessageCache` e, havendo divergência com `*messages.read()`, invocar `messages.set(...)` para renderizar a lista atualizada instantaneamente.

- **Decisão 2: Remoção do Bloqueio Rígido `len <= 50`:**
  - *Alternativa descartada:* Apenas alterar o limite mágico para `len <= 100`.
    - *Motivo do descarte:* Acoplamento frágil com o tamanho do lote de envelopes de primeiro nível.
  - *Escolha adotada:* Permitir a atualização reativa sempre que a lista de envelopes do cache divergir do sinal atual, garantindo que novas mensagens, remoções e alterações de flags de leitura reflitam com precisão na tela.

---

## 3. Contratos e Requisitos de Teste

1. **Reconciliação e Propagação de Envelopes:**
   - Teste automatizado comprovando que, após o ciclo de sincronização salvar novos envelopes no cache local, o leitor de sincronização UI recupera e atualiza o estado em memória.
2. **Consistência de Flags e Contagem:**
   - Verificação de que mensagens marcadas como lidas ou excluídas no ciclo refletem imediatamente na coleção de envelopes resultante.

---

## 4. Laudo de Resolução (Fase 9)

- **[Resumo]:** Atualização reativa e automática do sinal `messages` na interface após a conclusão do ciclo de sincronização em segundo plano.
- **[Solução]:** Introdução da função determinística `reconcile_ui_messages` em `domain/sync.rs`, eliminação do teto artificial arbitrário `len <= 50` em `ui/mod.rs` e recarregamento reativo imediato a partir dos envelopes reconciliados salvos no `LocalMessageCache` ao término de `sync_all_cycle`.
- **[Validação]:** Testes unitários dedicados em `auto_refresh_messages_test.rs` aprovados em 100%, suítes de regressão `local_date_and_sync_test` e `swr_test` íntegras, `cargo fmt --check` e `cargo clippy -p lauda-client --all-targets -- -D warnings` limpos.
