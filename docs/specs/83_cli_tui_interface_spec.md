# Especificação Técnica: Interface TUI Interativa para o Lauda

**ID do Ticket:** #83  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Em Andamento (Fase 3: Spec)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

O Lauda é um cliente e servidor de e-mail soberano com design minimalista. Embora disponha de interface web/desktop em Dioxus e aplicativo Android, usuários avançados de terminal e administradores de sistema necessitam de uma experiência interativa nativa de terminal (TUI - Terminal User Interface) ergonômica, rápida e focada em teclado, capaz de substituir clientes pesados ou dependentes de C como o *aerc* e o *mutt*.

O objetivo deste ticket é fornecer uma TUI nativa em Rust via subcomando `tui` no binário cliente do Lauda (`lauda-client tui`), permitindo navegar pelas caixas postais, listar mensagens com atualização instantânea, ler e-mails formatados no terminal e disparar buscas.

---

## 2. Requisitos Arquiteturais e Contratos

### 2.1. Ponto de Entrada da CLI
- O binário `lauda-client` deve aceitar o argumento/subcomando `tui`:
  - `lauda-client tui` (ou execução padrão sem flags gráficas ativas em terminal interativo).
- Inicialização do terminal via `crossterm` em modo raw com captura de eventos de teclado e alternate screen.
- Restauração garantida do terminal em caso de encerramento normal ou erro.

### 2.2. Máquina de Estados da TUI (`TuiApp`)
O estado da aplicação TUI deve ser desacoplado do loop de renderização para viabilizar testes unitários e determinísticos:
- **Telas / Modos Ativos**:
  - `TuiScreen::MailboxList`: Painel lateral ou seletor de caixas postais (`INBOX`, `Sent`, `Archive`, `Trash`).
  - `TuiScreen::EnvelopeList`: Lista de mensagens da caixa ativa com remetente, assunto, data e flags (`unread`, `flagged`).
  - `TuiScreen::MessageReader`: Leitor com cabeçalhos trifocais e corpo da mensagem formatado em texto puro/markdown.
  - `TuiScreen::Search`: Campo de busca rápida no topo com filtragem instantânea nos envelopes.
- **Atalhos Ergonômicos (Estilo Vim/Aerc)**:
  - `j` / `↓`: Navegar para baixo.
  - `k` / `↑`: Navegar para cima.
  - `Enter`: Abrir caixa postal ou abrir leitor de mensagem.
  - `q` / `Esc`: Voltar à tela anterior ou sair da TUI se estiver no nível raiz.
  - `/`: Iniciar busca instantânea.
  - `m`: Alternar foco entre painel de pastas e lista de mensagens.
  - `r`: Recarregar mensagens da caixa ativa.

### 2.3. Integração com o Ecossistema Lauda
- Utiliza a configuração de sessão existente (`SessionConfig`, `load_session`).
- Consome o cache local SQLite (`LocalMessageCache`) para inicialização com latência zero (0ms).
- Conecta-se à API HTTP (`HttpApiClient`) quando online para revalidação e busca de dados.

---

## 3. Critérios de Aceite

1. Comando `lauda-client tui` disponível e funcional em plataformas nativas (Linux/macOS/BSD).
2. Navegação fluida por caixas postais e mensagens através de atalhos de teclado `j`/`k`/`Enter`/`q`/`/`.
3. Visualizador de mensagens exibindo cabeçalhos e corpo formatado sem quebra ou corrupção de layout no terminal.
4. Arquitetura testável com suíte automatizada validando transições de estado, processamento de teclas e renderização determinística.
5. Conformidade estrita Gosplan (`cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`).

---

## 4. Registro de Fases DD-TDD

- [x] **Fase 1 (Ticket):** Ticket #83 formalizado e delimitado.
- [x] **Fase 2 (Branch):** Branch `feat/83-cli-tui-interface` criada a partir da `main`.
- [x] **Fase 3 (Spec):** Especificação técnica formalizada neste documento.
- [ ] **Fase 4 (Red Test):** Testes unitários falhos para máquina de estados e processamento de eventos da TUI.
- [ ] **Fase 5 (Green Implement):** Implementação da máquina de estados, renderizador Ratatui e subcomando CLI.
- [ ] **Fase 6 (Refactor):** Código modularizado, idiomático e com zero warnings.
- [ ] **Fase 7 (Mutation Test):** Eliminação de mutantes de navegação e limites de seleção.
- [ ] **Fase 8 (Quality Gate):** Validação com `cargo fmt --check`, `cargo clippy` e suíte de testes.
- [ ] **Fase 9 (Finish & Close):** Merge via CCCP e fechamento do ticket #83.
