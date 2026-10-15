# Especificação Técnica: Banner de Atualização Disponível Dispensável

**ID do Ticket:** #56  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Contexto e Motivação

No cliente Lauda (`lauda-client/src/ui/mod.rs`), ao detectar uma nova versão disponível remotamente (`UpdateStatus::Available`), o banner Trash Polka de "NOVA VERSÃO DISPONÍVEL" é renderizado de maneira estática no topo da interface. Atualmente, o banner não possui nenhum botão de dispensa (`✕`), permanecendo visível indefinidamente e ocupando espaço vertical valioso em dispositivos móveis, mesmo quando o usuário opta por não atualizar no momento.

---

## 2. Contrato de Interface e Comportamento

1. **Estado de Dispensa Reativo:**
   - O componente raiz `App` em `lauda-client/src/ui/mod.rs` gerencia um sinal reativo `dismissed_update_version: Signal<Option<String>>`.
   - Inicialmente o valor é `None`.
2. **Exibição Condicional:**
   - O banner de atualização só é renderizado se:
     - `update_notice` contiver um manifesto; E
     - A versão desse manifesto for diferente de `dismissed_update_version`.
3. **Elemento de Fechamento (Botão `✕`):**
   - Inserido no canto direito do banner de atualização, adjacente ao link "ATUALIZAR APK".
   - Estilizado na estética Trash Polka (`background: #0a0a0a; color: #fff; border: 1px solid #0a0a0a; cursor: pointer; padding: 0.3rem 0.6rem; font-weight: bold;`).
   - Ao ser clicado, atualiza `dismissed_update_version.set(Some(manifest.version.clone()))`, ocultando o banner instantaneamente sem recarregar a tela.
4. **Resiliência:**
   - Caso um novo manifesto com versão diferente e superior seja publicado em tempo de execução, o banner pode ser reexibido para aquela nova versão.

---

## 3. Critérios de Aceite e Verificação

1. **Botão de Fechamento:** O banner contém botão interativo de dispensa com identificador claro (`✕` / `DISPENSAR`).
2. **Ocultação Instantânea:** O clique no botão remove o banner da visualização sem afetar a navegação ou estado da caixa de correio.
3. **Cobertura de Testes:** Testes automatizados verificando a lógica de dispensa e persistência em memória do sinal.
4. **Qualidade e Estilo:** Conformidade com `cargo fmt -- --check`, `cargo clippy -- -D warnings` e `cccp check --strict`.
