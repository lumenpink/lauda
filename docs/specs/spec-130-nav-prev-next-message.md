# Especificação Técnica - Ticket #130: Navegação Próximo e Anterior no Leitor de Mensagens

## 1. Contexto & Motivação
Ao abrir um e-mail para leitura, a ausência de botões para avançar ou retroceder forçava o usuário a retornar repetidamente para a lista principal da caixa de entrada (`Screen::Inbox`) e selecionar manualmente o e-mail seguinte.
Esta fricção prejudica fluxos diários de triagem rápida de caixa de entrada em qualquer cliente de e-mail moderno.

## 2. Decisões de Arquitetura & UX
1. **Função Pura de Resolução de Vizinhos (`find_neighbor_uids`):**
   - Localizada em `lauda-client/src/domain/view.rs`.
   - Assinatura: `pub fn find_neighbor_uids(envelopes: &[MessageEnvelope], current_uid: u32) -> (Option<u32>, Option<u32>)`.
   - O primeiro elemento (`prev`) corresponde a `envelopes[pos - 1]`.
   - O segundo elemento (`next`) corresponde a `envelopes[pos + 1]`.
   - Retorna `None` nas bordas (início e fim da lista) ou se o UID atual não estiver presente.
2. **Interface Visual no Leitor (`MessageView`):**
   - Inseridos botões dedicados `◀ ANTERIOR` e `PRÓXIMO ▶` na barra de ações.
   - Quando `prev` ou `next` for `None`, o botão correspondente é desabilitado (`disabled: true`) e estilizado com baixa opacidade e cursor padrão (`not-allowed`), mantendo o layout estável.
   - Ao clicar em `◀ ANTERIOR` ou `PRÓXIMO ▶`:
     - Dispara o carregamento atômico da nova mensagem via cache local (SQLite / IndexedDB) ou fallback de rede se offline/não baixado.
     - Aplica optimistic update marcando a nova mensagem como lida e enfileirando ação na fila persistente offline-first.
     - Atualiza `active_message` e altera o estado da tela para `Screen::MessageDetail(target_uid)`.
     - Reseta o modo de visualização para `SIMPLIFICADO`.

## 3. Plano de Testes & Validação
- **Red Test:** Teste automatizado em `client_test.rs` chamando `find_neighbor_uids` com lista de envelopes simulada e validando limites de borda.
- **Green Implement:** Implementar `find_neighbor_uids` e integrar os botões na barra de ferramentas de `MessageView`.
- **Mutation Test:** Testes de mutação para lista vazia, UID não encontrado, lista de tamanho 1, e lista com múltiplos itens.
- **Quality Gate:** `cargo fmt --check`, `clippy -D warnings` e verificação WebAssembly.

## 4. Status de Resolução
- Concluído e verificado em 2026-11-23. Integrado com sucesso no leitor de mensagens.
