# Especificação Técnica - Ticket #129: Alternador de Modo no Leitor de Mensagens e Renomeação para SIMPLIFICADO

## 1. Contexto & Motivação
Anteriormente, o seletor do modo de visualização (`ViewMode`) ficava no cabeçalho global da aplicação (`App` em `mod.rs`), impondo um estado global persistido a todas as mensagens.
Essa abordagem apresentava problemas ergonômicos:
1. O usuário precisava alternar o modo no topo da tela antes ou durante a leitura, afetando futuras mensagens abertas inadvertidamente.
2. O nome `TXT AUTO` era excessivamente técnico e gerava dúvida sobre o que realmente significava (renderização limpa/processada em Markdown vs texto puro).
3. A cada mensagem aberta, o usuário espera por padrão ver a versão limpa e legível (`SIMPLIFICADO`), podendo pontualmente alternar para `ORIGINAL` ou `HTML` apenas para inspecionar aquela mensagem específica, sem poluir a navegação de outras mensagens.

## 2. Decisões de Arquitetura & UX
1. **Remoção do Cabeçalho Global:**
   - O botão trifásico de alternância no header superior (`App`) é removido.
   - O estado global de `current_view_mode` no nível do `App` deixa de controlar o leitor de mensagens.
2. **Escopo Local por E-mail:**
   - Na tela do leitor de mensagem (`MessageView`), o modo passa a ser um estado local (`use_signal(|| ViewMode::TxtAuto)`).
   - Ao abrir qualquer mensagem, o modo sempre inicia como `ViewMode::TxtAuto` (rotulado como `SIMPLIFICADO ⟳` ou `SIMPLIFICADO`).
   - Se o usuário alternar para `HTML` ou `ORIGINAL`, a alteração é válida exclusivamente para aquela mensagem durante a sessão de leitura atual. Ao voltar para a Inbox ou abrir outro e-mail, a nova janela inicia automaticamente em `SIMPLIFICADO`.
3. **Renomeação do Rótulo:**
   - O método `ViewMode::label()` em `lauda-types` passa a retornar `"SIMPLIFICADO ⟳"` para `ViewMode::TxtAuto`, mantendo `"ORIGINAL ⟳"` para `ViewMode::TxtOriginal` e `"HTML ⟳"` para `ViewMode::Html`.
4. **Posicionamento no Leitor:**
   - O botão é posicionado na barra de ferramentas superior do `MessageView`, junto aos botões de ação rápida, com destaque visual consistente com a identidade Trash Polka do projeto.

## 3. Plano de Testes & Validação
- **Red Test (Contrato):**
  - Testar `ViewMode::label()` garantindo que retorna `"SIMPLIFICADO ⟳"` para `ViewMode::TxtAuto`.
  - Testar o ciclo `next()`: `TxtAuto` -> `Html` -> `TxtOriginal` -> `TxtAuto`.
  - Testar serialização/desserialização serde de `ViewMode` mantendo compatibilidade retroativa com `"txt_auto"`.
- **Quality Gate:**
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo check --target wasm32-unknown-unknown -p lauda-client`
