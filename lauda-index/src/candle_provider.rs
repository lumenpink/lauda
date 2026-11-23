use std::path::Path;

/// Provedor de embeddings neurais em CPU utilizando o framework Candle (Rust puro).
/// Implementa a arquitetura Bert/XLM-RoBERTa do modelo multilingual-e5-small.
#[derive(Debug)]
pub struct CandleEmbeddingProvider;

impl CandleEmbeddingProvider {
    /// Carrega o modelo a partir de um diretório contendo model.safetensors e tokenizer.json.
    pub fn load<P: AsRef<Path>>(_path: P) -> Result<Self, String> {
        // Red test: falha propositalmente antes da Fase 5
        Err("CandleEmbeddingProvider ainda não implementado".to_string())
    }
}
