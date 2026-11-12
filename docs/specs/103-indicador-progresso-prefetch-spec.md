# Especificação Técnica: Indicador de Progresso Decrementável no Prefetch de Mensagens

**ID do Ticket:** #103  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Concluído (Fase 9)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Durante os testes em dispositivo móvel com a versão APK, a usuária relatou:
> "Ele apresentou uma mensagem de sincronizando 37 mensagens. Ficou quase 1 minuto parado nessa mensagem até que apareceu sincronizado. O contador não foi diminuindo com o tempo."

### 1.1 Diagnóstico
- O cliente calculava a quantidade de mensagens faltantes (`count_missing_bodies`) antes do ciclo e atribuía `sync_pending_count.set(37)`.
- A função `prefetch_missing_bodies` realizava o download e salvamento em lotes concorrentes (chunks de 5 itens), porém não emitia nenhum evento intermediário de telemetria.
- Somente após o término integral do download de todos os lotes e do término de contatos e agenda é que `sync_pending_count.set(0)` era executado.
- Como resultado, a interface exibia `↻ Sincronizando (37 restantes)` estático durante todo o processo, gerando a falsa impressão de travamento.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: Callback Granular por Lote Processado:**
  - *Alternativa descartada:* Re-renderizar a cada mensagem individual ou polling da UI via timer.
    - *Motivo do descarte:* Como as requisições ocorrem em lotes paralelos (`BatchJoin`), disparar re-render a cada fração de milissegundo causa overhead de layout.
    - *Escolha adotada:* Notificar o callback `on_progress(remaining)` ao término de cada lote concorrente (chunk de até 5 mensagens), decrementando o valor de forma fluida (ex: 37 -> 32 -> 27 ... -> 0).

- **Decisão 2: Preservação de Retrocompatibilidade:**
  - *Alternativa descartada:* Mudar a assinatura existente de `prefetch_missing_bodies` e `sync_all_cycle` quebrando testes anteriores.
    - *Motivo do descarte:* Quebraria testes unitários e de integração existentes no repositório.
    - *Escolha adotada:* Introduzir `prefetch_missing_bodies_with_progress` e `sync_all_cycle_with_progress`, mantendo as funções clássicas como wrappers sem efeito colateral com closure vazia `|_| {}`.

- **Decisão 3: Integração no Sinal da UI:**
  - Em `lauda-client/src/ui/mod.rs`, passar a closure `|remaining| sync_pending_count.set(remaining)` para o ciclo de sincronização, garantindo que o status no cabeçalho reflita a convergência incremental.

---

## 3. Contratos e Requisitos de Teste

1. **Decrementação Progressiva:**
   - Teste automatizado comprovando que `prefetch_missing_bodies_with_progress` invoca o callback com valores estritamente decrescentes até chegar a 0.
2. **Integração no Ciclo Unificado:**
   - Teste comprovando que `sync_all_cycle_with_progress` propaga o progresso de prefetch para o callback fornecido.

---

## 4. Laudo de Resolução (Fase 9)

- **[Resumo]:** Adição de telemetria e callback de progresso decrementável em tempo real no prefetch de mensagens (`prefetch_missing_bodies_with_progress` e `sync_all_cycle_with_progress`).
- **[Solução]:** Implementação de emissão progressiva do quantitativo restante de mensagens a cada lote processado com sucesso, preservando total retrocompatibilidade via wrappers funcionais e ligando a emissão diretamente ao sinal `sync_pending_count` da interface do usuário.
- **[Validação]:** Testes dedicados em `prefetch_progress_test.rs` aprovados em 100%, suítes de regressão `background_sync_worker_test`, `eager_body_sync_test` e `auto_refresh_messages_test` íntegras, `cargo fmt --check` e `cargo clippy -p lauda-client --all-targets -- -D warnings` limpos.
