# Especificação Técnica - Ticket #116
## Integração do Motor Neural de Embeddings Multilingual-E5-Small em CPU (`lauda-server` / `lauda-index`)

### Contexto & Motivação
A Seção 18 da especificação formal (`SPEC.md`) estabelece que o motor semântico do Lauda deve utilizar o modelo neural canônico `multilingual-e5-small` (384 dimensões em ponto flutuante, normalizado pela norma L2, operando 100% na CPU com 0 MB de VRAM).
No entanto, a implementação prévia em `lauda-index` restringia o `DynamicEmbeddingProvider` à variante `Fallback(SimpleCpuEmbeddingProvider)`, um mock sintético com apenas 6 conceitos hardcoded e dispersão de trigramas.
Isso impedia que e-mails reais no `lauda-server` fossem indexados com representação vetorial genuína, quebrando buscas por linguagem natural cotidiana e sinonímia.

### Árvore de Decisão
- **Alternativa A (Descartada):** Dependência incondicional direta de `fastembed` / `ort` em `lauda-index`.
  - *Motivo do descarte:* Quebra fatal de compilação cruzada no WebAssembly (`wasm32-unknown-unknown`) e em testes herméticos de CI que operam offline. Natasha não conseguiria rodar a esteira nem compilar o cliente sem os SDKs de C/C++ do ONNX Runtime.
- **Alternativa B (Descartada):** Manter o mock de trigramas e apenas aumentar o dicionário para centenas de palavras.
  - *Motivo do descarte:* Dívida técnica inaceitável. O espaço latente de hash de palavras não possui continuidade semântica nem generalização para novas frases e idiomas.
- **Alternativa C (Escolhida):** Arquitetura desacoplada via feature flag `neural` e suporte a provedores dinâmicos injetáveis:
  1. No `lauda-index`: `DynamicEmbeddingProvider` ganha a variante `Neural(Arc<dyn EmbeddingProvider>)`, permitindo plugar motores de inferência e mocks estruturados em testes herméticos.
  2. A feature flag opcional `neural` habilita `fastembed` com `EmbeddingModel::MultilingualE5Small` sob demanda em plataformas nativas de servidor.
  3. Preservação do fallback determinístico em CPU caso pesos locais não estejam acessíveis, garantindo tolerância a falhas e zero crash em ambientes sem rede.

### Requisitos Técnicos
1. `DynamicEmbeddingProvider` suporta `new_neural(Arc<dyn EmbeddingProvider>)` e construtores dinâmicos com verificação de arquivo de modelo.
2. Dimensão vetorial estritamente padronizada em 384 floats normalizados ($L_2 = 1.0$).
3. Preservação estrita das instruções assimétricas E5 (`query: ` para buscas e `passage: ` para segmentos indexados via Urupema).
4. Crate `lauda-index` compila tanto sem a feature `neural` (padrão hermético/WASM) quanto com a feature ativa.
