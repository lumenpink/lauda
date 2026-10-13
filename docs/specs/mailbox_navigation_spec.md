# Especificação Técnica: Redesign e Apresentação de Pastas de Correio

- **Ticket:** #30 (`feat(ui): redesign e apresentação de pastas de correio`)
- **Fase DD-TDD:** Fase 3 (Spec)
- **Status:** Aprovado para implementação

---

## 1. Visão Geral e Objetivos

Atualmente no `lauda-client`, a seleção de caixas de correio (`mailboxes`) é exibida em uma linha horizontal de botões flex no cabeçalho da visualização de entrada (`InboxView`). À medida que o usuário possui múltiplas pastas e subpastas, esse layout quebra linhas de forma desordenada e consome espaço vertical valioso.

Esta especificação define:
1. **Estrutura Hierárquica e Normalização:** Mapeamento de caminhos de caixas de correio IMAP (`INBOX/Projetos/X`, `Work.Clients`, etc.) em uma árvore/lista de nós com profundidade, ordenação canônica e ícones semânticos.
2. **Layout Responsivo Trash Polka:**
   - **Visualizador Colunar / Sidebar (Desktop ≥ 768px):** Painel lateral compacto com divisão clara de filtros e pastas.
   - **Drawer Retrátil (Mobile < 768px):** Acionado por botão de menu brutalista (`☰ PASTAS`), com backdrop escuro/vermelho carmesim e fechamento automático ao selecionar pasta ou clicar fora.
3. **Seções de Navegação:**
   - **Filtros Rápidos / Pastas Virtuais:** Todos, Não Lidos, Favoritos (★), Com Anexos (📎).
   - **Pastas do Sistema / Usuário:** Caixa de entrada, Enviados, Rascunhos, Lixeira, Spam, Arquivo e subpastas hierárquicas.
4. **Badges Brutalistas Trash Polka:**
   - Pastas com não lidos: Destaque carmesim `[#dc143c]` (`[3 / 120]`).
   - Pastas com zero não lidos: Monocromático cinza escuro/preto (`[120]`).
   - Pasta ativa: Fundo preto `#0a0a0a`, texto branco `#ffffff`, borda espessa e sombra deslocada.

---

## 2. Modelo de Dados & Algoritmo de Hierarquia

### 2.1. Representação do Nó de Pasta (`FolderNode`)

Em `lauda-client/src/domain/folders.rs` (ou `mailbox_tree.rs`):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderRole {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Spam,
    Archive,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderNode {
    pub raw_name: String,      // Nome completo original no servidor IMAP (ex: "INBOX/Projetos/Lauda")
    pub display_name: String,  // Nome exibido (ex: "Lauda" ou "📥 Caixa de Entrada")
    pub role: FolderRole,
    pub depth: usize,          // Nível de profundidade (0 para raiz, 1 para subpasta, etc.)
    pub total_messages: u32,
    pub unread_messages: u32,
}
```

### 2.2. Algoritmo de Mapeamento e Ordenação
1. Identificar delimitador comum (`/` ou `.`) presente nos nomes.
2. Reconhecer papéis canônicos padrão IMAP (`INBOX`, `Sent`, `Drafts`, `Trash`, `Junk`/`Spam`, `Archive`), atribuindo prioridade de ordenação fixa (Inbox primeiro, depois Sent, Drafts, Archive, Spam, Trash, e então pastas customizadas em ordem alfabética hierárquica).
3. Gerar lista ordenada de `FolderNode` com cálculo de `depth` e prefixos visuais apropriados (`└── ` ou indentação padding).

---

## 3. Componentes da Interface (UI)

1. `MailboxDrawer` / `MailboxSidebar`:
   - Componente reutilizável para exibição dos filtros rápidos e da lista hierárquica de pastas.
   - Suporta modo desktop (fixado à esquerda como coluna flex) e modo mobile (drawer posicionado com `position: fixed; inset: 0; z-index: 100`).
   - Botão de recolher/fechar `✕ FECHAR` e controle de estado reativo `is_drawer_open: Signal<bool>`.
2. Integração no `InboxView`:
   - Substituição da barra horizontal confusa pelo botão de alternância `☰ PASTAS ({active_box_name})` e pelo layout colunar responsivo.
   - Manutenção de total compatibilidade com os sinais Dioxus existentes (`mailboxes`, `current_mailbox`, `messages`).

---

## 4. Critérios de Aceitação & Testes

1. `parse_mailbox_tree`:
   - Lista desordenada contendo `INBOX`, `INBOX/Dev`, `INBOX/Dev/Rust`, `Trash`, `Sent` deve ser convertida para nós ordenados de acordo com as regras de prioridade e profundidades corretas (`0`, `1`, `2`, `0`, `0`).
   - Delimitadores `.` e `/` são tratados corretamente.
   - Contadores `total_messages` e `unread_messages` são preservados sem perda.
2. Formatação de Badges:
   - Contadores com não lidos geram badge carmesim com contagem detalhada.
   - Contadores sem não lidos geram badge simples.
3. Testes de Renderização / Componente no Dioxus:
   - O componente compila em Desktop e WASM sem warnings.
   - Clipes de clippy e formatação 100% limpos.
