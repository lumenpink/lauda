# Spec: Integração Urupema 0.1.1 e Reindexação Vetorial no Lauda Server (#75)

## Contexto
Ao converter e-mails ricos recebidos em formato HTML para Markdown (`txt_auto`), mensagens de newsletters (ex.: GitHub Universe, Canal Meio) geravam poluentes visuais como sequências contínuas de underlines (`___`) e divisores ASCII corrompidos.
O Urupema 0.1.1 introduziu o sanitizador determinístico `sanitize_markdown_noise` e a heurística de seleção `is_markdown_empty_or_polluted`.
Adicionalmente, como o banco de vetores SQLite (`indexed_documents`) é incremental (`reconcile_mailbox` apenas adiciona novos UIDs), é indispensável um mecanismo de reindexação semântica forçada para recalcular os vetores das mensagens existentes a partir dos textos limpos.

## Objetivo
1. Atualizar a dependência da crate `urupema` para a versão `0.1.1` do registro Gambi em `lauda-server` e `lauda-index`.
2. Em `lauda-server/src/adapters/maildir.rs`, aplicar `urupema::domain::sanitize_markdown_noise` na geração do `txt_auto`, expurgando linhas divisórias de underlines e ruídos de newsletters.
3. Aplicar fallback para `txt_original` caso `is_markdown_empty_or_polluted` detecte degradação do Markdown comparado ao texto puro original.
4. Implementar rotina de reindexação semântica forçada (`reindex_all_semantic` ou comando de expurgação do índice SQLite) para recalcular os vetores dos e-mails da Maildir.
5. Garantir que a suíte de testes de integração e testes E2E validem a higienização do texto e o recálculo dos vetores.

## Critérios de Aceitação
- Dependência `urupema` travada em `0.1.1` (`registry = "gambi"`) no workspace.
- Teste de unidade/integração validando que uma mensagem contendo divisores `____________________` e linhas de `___` resulte em `txt_auto` completamente limpo.
- Teste validando que a rotina de reindexação substitui e recalcula vetores para UIDs já previamente persistidos no SQLite.
- Quality Gate: `cargo fmt`, `cargo clippy` com zero warnings e testes do workspace passando.
