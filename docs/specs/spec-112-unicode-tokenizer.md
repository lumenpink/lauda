# Especificação Técnica - Ticket #112
## Normalização Canônica Unicode no Tokenizer (`lauda-index`)

### Contexto & Motivação
O motor de busca do Lauda (`lauda-index`) depende da função `normalize` em `tokenizer.rs` para extrair termos de e-mails recebidos e indexá-los no índice invertido (BM25).
Anteriormente, a função realizava `c.to_ascii_lowercase()` antes de casar os caracteres com acentos em minúsculas (`'á'`, `'õ'`, `'ç'`, etc.). Como caracteres acentuados maiúsculos (`'Á'`, `'Õ'`, `'Ç'`) estão fora da faixa ASCII `[A-Z]`, eles não sofriam conversão para minúsculas e caíam no braço de escape `other => out.push(other)`.

Isso gerava tokens corrompidos com caracteres maiúsculos mistos (ex.: `"ATENÇÃO"` virava `"atenÇÃo"`), tornando impossível a localização de e-mails em caixa alta quando o usuário pesquisava por `"atencao"`.

### Árvore de Decisão
- **Alternativa A (Descartada):** Mapear manualmente caracteres maiúsculos no `match` (`'Á' | 'À' | 'á' | ... => out.push('a')`).
  - *Motivo do descarte:* Frágil e propenso a esquecimentos; não escala e duplica a tabela de correspondência. Se Natasha precisar adicionar novos símbolos ou idiomas no futuro, terá que duplicar regras.
- **Alternativa B (Descartada):** Adicionar dependência pesada de desacentuação Unicode (ex.: `unicode-normalization` ou `deunicode`).
  - *Motivo do descarte:* Inchaço de dependências em um crate de base (`lauda-index`) que compila para WebAssembly com restrições severas de tamanho e tempo de build.
- **Alternativa C (Escolhida):** Iterar sobre `c.to_lowercase()` (suporte nativo e completo a minúsculas Unicode no Rust `std`), e em seguida aplicar a redução diacrítica canônica para o alfabeto latino de 7 bits (`a-z`, `c`).
  - *Vantagens:* Zero dependências externas adicionais, preserva compatibilidade estrita com WebAssembly (`wasm32-unknown-unknown`), velocidade $O(N)$ em tempo de execução e garantia de que qualquer glifo maiúsculo passe pela redução de acentos.

### Requisitos e Casos de Borda
1. Frases inteiramente em maiúsculas (`"ATENÇÃO: REUNIÃO DE ORÇAMENTO"`) devem ser tokenizadas como `["atencao", "reuniao", "de", "orcamento"]`.
2. Todos os diacríticos latinos padrão em português (`á, à, ã, â, ä, é, è, ê, ë, í, ì, î, ï, ó, ò, õ, ô, ö, ú, ù, û, ü, ç`) devem ser reduzidos às formas base (`a, e, i, o, u, c`), independentemente de serem maiúsculos ou minúsculos no texto de origem.
3. Caracteres alfanuméricos ASCII normais devem ser preservados em caixa baixa (`A-Z` -> `a-z`, `0-9` preservados).

### Conclusão & Validação (Fase 9)
- Normalização Unicode implementada com sucesso com `c.to_lowercase()`.
- Testes unitários, de mutação e quality gate (`cargo fmt`, `cargo clippy`) 100% aprovados.

