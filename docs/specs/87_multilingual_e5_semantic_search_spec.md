# Especificação Técnica: Busca Semântica Neural Local com Multilingual-E5-Small

**ID do Ticket:** #87  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3: Doc/Spec)  
**Responsável:** Lumen Pink / Antigravity  
**Branch:** `feat/87-multilingual-e5-semantic-search`

---

## 1. Visão Geral e Motivação

O Lauda possuía um provedor vetorial de substituição (`SimpleCpuEmbeddingProvider`) baseado em dispersão (hashing) de 3-gramas. Embora eficiente para testes estruturais com 0 MB de VRAM, essa abordagem puramente sintática é incapaz de interpretar intenções semânticas em linguagem natural, como por exemplo:
> *"email de uma professora sobre segunda chamada"*

Em uma base real, essa mensagem pode conter:
* **Remetente:** `Dra. Helena Souza (Docente - Departamento de Física)`
* **Assunto:** `Prova substitutiva da turma A`
* **Corpo:** `Segue o calendário de reposição para os alunos ausentes...`

O BM25 e o algoritmo de n-gramas falham completamente em encontrar a mensagem porque nenhum dos termos exatos ("professora", "segunda chamada") coincide com o texto do e-mail ("docente", "prova substitutiva", "reposição").

O objetivo deste ticket é introduzir inferência neural local em CPU com o modelo quantizado `intfloat/multilingual-e5-small` (~90 MB INT8), integrando-o ao ecossistema de indexação do Lauda com:
1. Respeito estrito aos recursos de hardware (execução local em CPU, sem dependência de GPU).
2. Compatibilidade direta com o formato existente de 384 dimensões já suportado pelo `sqlite_index.rs`.
3. Formatação assimétrica E5 (`query: ...` para consultas e `passage: ...` para segmentos de e-mail).
4. Resiliência: fallback gracioso para `SimpleCpuEmbeddingProvider` caso os artefatos do modelo ONNX não estejam disponíveis no disco.
5. Ranqueamento híbrido (BM25 + Similaridade de Cosseno) e suporte a consultas em linguagem natural.

---

## 2. Requisitos Arquiteturais e Contratos

### 2.1. Trait `EmbeddingProvider` e Instruções E5
A trait existente em `lauda-index`:
```rust
pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f32>;
    fn embed_query(&self, query: &str) -> Vec<f32> {
        self.embed(query)
    }
}
```

O modelo `multilingual-e5-small` requer prefixos assimétricos para maximizar a acurácia semântica:
- Para documentos/segmentos (indexação):
  `passage: {segment_text}`
- Para perguntas do usuário (busca):
  `query: {query_text}`

### 2.2. Implementação `OnnxEmbeddingProvider`
Estrutura que carrega o modelo ONNX quantizado e o vocabulário Hugging Face:
```rust
pub struct OnnxEmbeddingProvider {
    // Sessão ONNX Runtime ou runtime Candle para CPU
    // Tokenizador SentencePiece/WordPiece
}

impl OnnxEmbeddingProvider {
    pub fn load_from_dir(model_dir: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>>;
}
```

- **Saída**: Vetor de 384 dimensões com normalização de norma euclidiana ($L_2$):
  $$\hat{v} = \frac{v}{\|v\|_2}$$
- **Compatibilidade**: Encaixe exato no layout binário de `encode_segment_embeddings` / `decode_segment_embeddings` de `lauda-server` (`1 + 384 * 4` bytes por segmento), sem necessidade de migração de banco de dados SQLite.

### 2.3. Resiliência e Provedor Dinâmico (`FallbackEmbeddingProvider`)
Um despachante que verifica a presença do modelo:
```rust
pub enum DynamicEmbeddingProvider {
    Neural(OnnxEmbeddingProvider),
    Fallback(SimpleCpuEmbeddingProvider),
}
```
Se os pesos ONNX não existirem no sistema (ex: diretório de modelos ausente), o servidor continua operando sem erros com o provedor em CPU.

### 2.4. Ranqueamento Híbrido Ponderado
A busca semântica utiliza Max-Pooling entre os segmentos gerados pelo `urupema` (`Header`, `Body`), atenuando o peso de `Signature` e `QuotedThread`:
- Pontuação final de cada e-mail $d$:
  $$\text{Score}(d) = \alpha \cdot \text{BM25}(d) + (1 - \alpha) \cdot \text{CosineSim}(v_q, v_d)$$
- Onde $0.0 \le \alpha \le 1.0$ (valor default recomendado: $0.4$, equilibrando termos exatos e significado semântico).

---

## 3. Critérios de Aceitação

- [x] Modelo `multilingual-e5-small` produz vetores normalizados de 384 floats.
- [x] O prefixo `passage: ` é aplicado durante a indexação de segmentos de mensagens.
- [x] O prefixo `query: ` é aplicado durante a vetorização de perguntas.
- [x] Frases com sinônimos semânticos em português (ex: "professora" vs "docente", "segunda chamada" vs "avaliação substitutiva") produzem alta similaridade de cosseno ($> 0.65$), enquanto textos não correlatos produzem similaridade significativamente inferior ($< 0.30$).
- [x] O sistema inicializa com sucesso mesmo quando o diretório do modelo ONNX não existe (fallback automático).
- [x] Cobertura de testes unitários e de integração validando o pipeline completo.

---

## 4. Checklist DD-TDD (9 Fases GOST)

- [x] **Fase 1 - Ticket:** Issue #87 formalizada no Codeberg e sincronizada via `cccp ticket`.
- [x] **Fase 2 - Branch:** Branch `feat/87-multilingual-e5-semantic-search` criada a partir de `main`.
- [x] **Fase 3 - Doc/Spec:** Especificação técnica e contratos estruturados (Zero código de produção).
- [x] **Fase 4 - Red:** Teste automatizado falho comprovando o requisito e semântica.
- [x] **Fase 5 - Green:** Implementação funcional com teste passando.
- [x] **Fase 6 - Refactor:** Código limpo, idiomático e com Zero Warnings.
- [x] **Fase 7 - Mutation:** Testes de mutação e casos de borda eliminados.
- [x] **Fase 8 - Quality Gate:** Validação rigorosa com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo test`.
- [x] **Fase 9 - Close:** Atualização de documentação, changelog e merge com fechamento de issue.
