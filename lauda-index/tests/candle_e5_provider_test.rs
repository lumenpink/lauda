use lauda_index::candle_provider::CandleEmbeddingProvider;
use lauda_index::{DynamicEmbeddingProvider, E5InstructionProvider, EmbeddingProvider};

#[test]
fn test_candle_provider_load_fails_on_missing() {
    let result = CandleEmbeddingProvider::load("/caminho/completamente/inexistente/para/modelo");
    assert!(
        result.is_err(),
        "Carregamento deve reportar erro com caminho inexistente"
    );
}

#[test]
fn test_dynamic_embedding_provider_fallback_when_candle_missing() {
    let provider = DynamicEmbeddingProvider::from_path_or_fallback("/caminho/inexistente");
    assert!(provider.is_fallback());
    assert!(!provider.is_neural());

    let emb = provider.embed("teste de busca");
    assert_eq!(emb.len(), 384);
}

#[test]
fn test_dynamic_embedding_provider_fallback_when_corrupted_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let model_path = temp_dir.path().join("model.safetensors");
    let tokenizer_path = temp_dir.path().join("tokenizer.json");
    std::fs::write(&model_path, b"dados_corrompidos_invalidos").unwrap();
    std::fs::write(&tokenizer_path, b"dados_corrompidos_invalidos").unwrap();

    // Deve recair graciosamente no fallback sem causar panic
    let provider = DynamicEmbeddingProvider::from_path_or_fallback(temp_dir.path());
    assert!(
        provider.is_fallback(),
        "Arquivos corrompidos devem acionar o fallback determinístico"
    );
    assert_eq!(provider.embed("consulta").len(), 384);
}

#[test]
fn test_e5_instruction_candle_embedding_contract() {
    let fallback = DynamicEmbeddingProvider::from_path_or_fallback("/caminho/inexistente");
    let e5 = E5InstructionProvider::new(fallback);
    let q = e5.embed_query("teste de consulta");
    let p = e5.embed_passage("teste de passagem");

    assert_eq!(q.len(), 384);
    assert_eq!(p.len(), 384);

    // Norma L2 deve ser 1.0 ou próxima em vetores não-nulos
    let q_norm: f32 = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    assert!(
        (q_norm - 1.0).abs() < 1e-4,
        "Norma L2 da query deve ser unitária: {q_norm}"
    );

    let p_norm: f32 = p.iter().map(|v| v * v).sum::<f32>().sqrt();
    assert!(
        (p_norm - 1.0).abs() < 1e-4,
        "Norma L2 da passagem deve ser unitária: {p_norm}"
    );
}
