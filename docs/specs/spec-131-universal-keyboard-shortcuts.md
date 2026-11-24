# Especificação Técnica - Ticket #131: Atalhos de Teclado Universais para Navegação e Ações em Web, GUI e APK

## 1. Contexto & Motivação
A usuária utiliza ativamente teclado físico no cliente web, em desktops e no aplicativo Android (APK).
A ausência de atalhos de teclado forçava o uso constante de ponteiro/toque para tarefas rotineiras de navegação, leitura, resposta e alternância de modo.

## 2. Decisões de Arquitetura & UX
1. **Domínio Puro (`ShortcutAction` e `parse_shortcut`):**
   - Implementado em `lauda-client/src/domain/view.rs`.
   - `pub fn parse_shortcut(key: &str, is_input_focused: bool) -> Option<ShortcutAction>`.
   - Regra estrita: se `is_input_focused == true`, nenhuma ação é disparada, preservando campos de busca, login e composição de e-mail.
   - Mapeamento ergonômico padrão vi/Gmail:
     - Navegação: `j` / `ArrowDown` (próximo), `k` / `ArrowUp` (anterior).
     - Abertura / Voltar: `Enter` / `o` (abrir), `u` / `Escape` (voltar à Inbox).
     - Ações rápidas: `c` (redigir), `r` (responder), `a` (responder a todos), `f` (encaminhar).
     - Gestão: `m` (alternar modo SIMPLIFICADO/HTML/ORIGINAL), `s` (favorito/estrela), `e` (arquivar), `#` / `Delete` (excluir).
2. **Camada de Captura Universal (Web, GUI e APK):**
   - Utiliza `dioxus::document::eval` acoplado ao `window.addEventListener('keydown', ...)` para capturar eventos em qualquer plataforma suportada (Navegador, WebView Android e Desktop).
   - O evento inspeciona se `document.activeElement` é `INPUT` ou `TEXTAREA` antes de enviar o sinal via `dioxus.send`.
   - As telas reagem atomicamente atualizando sinais reativos de tela (`Screen`), índice de navegação e carregamento de e-mails.

## 3. Plano de Testes & Validação
- **Red Test:** Testes de contrato em `client_test.rs` chamando `parse_shortcut` e comprovando ausência da implementação.
- **Green Implement:** Implementação do parser e dos handlers reativos.
- **Mutation Test:** Eliminação de mutantes em case-insensitivity (`J` vs `j`), variantes de teclas e bloqueio estrito em input focado.
- **Quality Gate:** `cargo fmt --check`, `clippy -D warnings` e verificação WebAssembly.
