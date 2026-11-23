use lauda_index::candle_provider::CandleEmbeddingProvider;
use lauda_index::{DynamicEmbeddingProvider, E5InstructionProvider, EmbeddingProvider};

#[test]
fn test_candle_provider_load_fails_on_missing_or_unimplemented() {
    let result = CandleEmbeddingProvider::load("/caminho/completamente/inexistente/para/modelo");
    assert!(
        result.is_err(),
        "Carregamento deve reportar erro com caminho inexistente ou na fase red"
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
fn test_e5_instruction_candle_embedding_contract() {
    let fallback = DynamicEmbeddingProvider::from_path_or_fallback("/caminho/inexistente");
    let e5 = E5InstructionProvider::new(fallback);
    let q = e5.embed_query("teste de consulta");
    let p = e5.embed_passage("teste de passagem");

    assert_eq!(q.len(), 384);
    assert_eq!(p.len(), 384);
}

#[test]
fn test_dynamic_embedding_provider_loads_neural_when_model_files_exist() {
    let temp_dir = tempfile::tempdir().unwrap();
    let model_path = temp_dir.path().join("model.safetensors");
    let tokenizer_path = temp_dir.path().join("tokenizer.json");
    std::fs::write(&model_path, b"mock").unwrap();
    std::fs::write(&tokenizer_path, b"mock").unwrap();

    let provider = DynamicEmbeddingProvider::from_path_or_fallback(temp_dir.path());
    assert!(
        provider.is_neural(),
        "Diretório com model.safetensors e tokenizer.json deve carregar variante neural"
    );
}
