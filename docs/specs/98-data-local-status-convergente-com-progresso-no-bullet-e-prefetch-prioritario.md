# Especificação Técnica: Data Local, Status Convergente com Progresso e Prefetch Prioritário

**ID do Ticket:** #98  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Em Implementação (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Atualmente, o cliente Lauda apresenta três limitações de experiência de uso e confiabilidade:
1. **Fuso horário cru:** As datas exibidas na lista de e-mails e nos detalhes da mensagem são renderizadas diretamente como strings literais retornadas pelo servidor (em UTC ou no fuso original do remetente), sem conversão para o horário local do dispositivo da usuária.
2. **Ausência de telemetria de convergência:** O indicador de status no cabeçalho resume-se a um binário estático `● ONLINE` vs `○ OFFLINE`. A usuária não tem visibilidade se o cliente terminou de baixar os corpos das mensagens, se está no meio de uma sincronização ou quantas mensagens ainda restam para atingir a convergência com o servidor.
3. **Prefetch desordenado ou atrasado:** Quando novas mensagens chegam, o prefetch de corpos nem sempre prioriza imediatamente as mais recentes em relação à data real de envio, causando atraso perceptível ao abrir um e-mail recém-chegado.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: Onde formatar e converter o fuso de data:**
  - *Alternativa descartada:* Converter no servidor (`lauda-server`).
    - *Motivo do descarte:* O servidor desconhece o fuso horário local do dispositivo de cada cliente (que pode mudar dinamicamente conforme viagens ou configurações de sistema do usuário). A fonte da verdade de metadados deve permanecer UTC/RFC 3339 no servidor e o cliente converte para o horário local na camada de apresentação.
  - *Escolha adotada:* Utilitário de formatação no cliente (`lauda-client/src/domain/time.rs`), compatível com Desktop, Android e WebAssembly (WASM), convertendo o timestamp para a zona local e formatando de forma contextual:
    - Mensagens de hoje: `HH:MM` (ex.: `14:35`)
    - Mensagens deste ano: `DD MMM HH:MM` (ex.: `10 nov 14:35`)
    - Mensagens de anos anteriores: `DD/MM/YYYY` (ex.: `10/11/2024`)

- **Decisão 2: Representação visual da convergência no cabeçalho:**
  - *Alternativa descartada:* Adicionar uma nova barra de progresso ou novos badges adicionais no topo.
    - *Motivo do descarte:* Em dispositivos móveis (telas de 360-400px), o espaço horizontal é escasso e causaria quebra de linhas indesejada no cabeçalho.
  - *Escolha adotada:* Reutilizar o bullet de status existente com três estados semânticos autoexplicativos:
    - `● Sincronizado` (verde): Online e convergido (0 pendências de corpo e 0 outbox).
    - `↻ Sincronizando (N)` (azul suave): Conectado, informando o número exato de corpos pendentes de download.
    - `○ Offline (N pendentes)` (cinza/alerta): Sem conectividade, indicando pendências represadas no cache local.

- **Decisão 3: Priorização estrita no prefetch de corpos:**
  - *Escolha adotada:* No método `prefetch_missing_bodies`, classificar a lista de pendências estritamente por timestamp Unix decrescente das mensagens (`env.date`), processando em lotes paralelos da mais recente para a mais antiga e atualizando o contador reativo de pendências a cada lote completado.

---

## 3. Requisitos e Contratos

1. **Formatador de Data Local (`format_local_date`):**
   - Entrada: string RFC 3339 / ISO 8601 ou unix timestamp.
   - Saída: string legível no fuso horário do dispositivo do usuário.
2. **Sinal Reativo de Pendências (`sync_pending_count`):**
   - O estado global da UI deve manter o número de mensagens pendentes de download na janela ativa.
   - O bullet deve refletir `● Sincronizado` se `pending == 0 && is_online`, `↻ Sincronizando ({pending})` se `pending > 0 && is_online`, e `○ Offline` se `!is_online`.
3. **Ordenação Temporal Estrita no Prefetch:**
   - Garantir ordenação por data decrescente dos envelopes antes de fatiar os chunks concorrentes de download de corpos.
4. **Conformidade de Testes:**
   - Teste unitário para conversão de data e formatação local.
   - Teste de prefetch prioritário por data decrescente.
   - `cccp check --strict` com 100% de aprovação.
