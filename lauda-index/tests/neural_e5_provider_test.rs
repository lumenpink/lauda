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
