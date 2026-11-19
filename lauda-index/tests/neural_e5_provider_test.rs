use lauda_index::{DynamicEmbeddingProvider, E5InstructionProvider, EmbeddingProvider};
use std::sync::Arc;

struct DummyNeuralProvider;

impl EmbeddingProvider for DummyNeuralProvider {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; 384];
        if text.contains("neural") {
            vec[0] = 1.0;
        }
        vec
    }
}

#[test]
fn test_dynamic_embedding_provider_neural_variant() {
    let dummy = Arc::new(DummyNeuralProvider);
    let dynamic = DynamicEmbeddingProvider::new_neural(dummy);

    assert!(dynamic.is_neural(), "Provedor deve reportar que é neural");
    assert!(
        !dynamic.is_fallback(),
        "Provedor não deve reportar que é fallback"
    );

    let e5 = E5InstructionProvider::new(dynamic);
    let vec_query = e5.embed_query("teste neural");
    assert_eq!(vec_query.len(), 384);
    assert_eq!(vec_query[0], 1.0);
}

#[test]
fn test_dynamic_embedding_provider_passage_and_fallback_mutation() {
    let dummy = Arc::new(DummyNeuralProvider);
    let dynamic = DynamicEmbeddingProvider::new_neural(dummy);
    let e5 = E5InstructionProvider::new(dynamic);

    // Valida delegação de passage com prefixo E5
    let vec_passage = e5.embed_passage("documento de teste neural");
    assert_eq!(vec_passage.len(), 384);
    assert_eq!(vec_passage[0], 1.0);

    // Valida que texto sem a palavra neural resulta em zeros
    let vec_empty = e5.embed_passage("documento comum sem correspondencia");
    assert_eq!(vec_empty.len(), 384);
    assert_eq!(vec_empty[0], 0.0);

    // Valida fallback quando o arquivo não existe
    let fallback = DynamicEmbeddingProvider::from_path_or_fallback("/var/lib/inexistente.onnx");
    assert!(fallback.is_fallback());
    assert!(!fallback.is_neural());
    assert_eq!(fallback.embed_query("consulta").len(), 384);
}
