use std::path::Path;

#[cfg(feature = "candle")]
use candle_core::{DType, Device, Tensor};
#[cfg(feature = "candle")]
use candle_nn::VarBuilder;
#[cfg(feature = "candle")]
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
#[cfg(feature = "candle")]
use tokenizers::Tokenizer;

use crate::index::EmbeddingProvider;

/// Provedor de embeddings neurais em CPU utilizando o framework Candle (Rust puro).
/// Implementa a arquitetura Bert/XLM-RoBERTa do modelo multilingual-e5-small
/// com Mean-Pooling ponderado pela attention mask e normalização Euclidiana L2.
pub struct CandleEmbeddingProvider {
    #[cfg(feature = "candle")]
    model: BertModel,
    #[cfg(feature = "candle")]
    tokenizer: Tokenizer,
    dim: usize,
}

impl std::fmt::Debug for CandleEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandleEmbeddingProvider")
            .field("dim", &self.dim)
            .finish()
    }
}

impl CandleEmbeddingProvider {
    /// Carrega o modelo a partir de um diretório contendo `model.safetensors` e `tokenizer.json`
    /// (ou a partir do caminho direto de um arquivo safetensors com tokenizer no mesmo diretório).
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        #[cfg(feature = "candle")]
        {
            let path = path.as_ref();
            let (weights_path, tokenizer_path, config_path) = if path.is_dir() {
                (
                    path.join("model.safetensors"),
                    path.join("tokenizer.json"),
                    path.join("config.json"),
                )
            } else {
                let parent = path.parent().unwrap_or_else(|| Path::new("."));
                (
                    path.to_path_buf(),
                    parent.join("tokenizer.json"),
                    parent.join("config.json"),
                )
            };

            if !weights_path.exists() {
                return Err(format!(
                    "Pesos do modelo não encontrados: {}",
                    weights_path.display()
                ));
            }
            if !tokenizer_path.exists() {
                return Err(format!(
                    "Tokenizer não encontrado: {}",
                    tokenizer_path.display()
                ));
            }

            let device = Device::Cpu;
            let config: BertConfig = if config_path.exists() {
                let data = std::fs::read_to_string(&config_path)
                    .map_err(|e| format!("Falha ao ler config.json: {e}"))?;
                serde_json::from_str(&data)
                    .map_err(|e| format!("Falha ao decodificar config.json: {e}"))?
            } else {
                BertConfig {
                    hidden_size: 384,
                    num_hidden_layers: 12,
                    num_attention_heads: 12,
                    intermediate_size: 1536,
                    vocab_size: 250002,
                    model_type: Some("roberta".to_string()),
                    ..Default::default()
                }
            };

            let dim = config.hidden_size;

            let tokenizer = Tokenizer::from_file(&tokenizer_path)
                .map_err(|e| format!("Falha ao carregar tokenizer: {e}"))?;

            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)
                    .map_err(|e| format!("Falha ao mapear safetensors: {e}"))?
            };

            let model = BertModel::load(vb, &config)
                .map_err(|e| format!("Falha ao carregar BertModel: {e}"))?;

            Ok(Self {
                model,
                tokenizer,
                dim,
            })
        }

        #[cfg(not(feature = "candle"))]
        {
            let _ = path;
            Err(
                "Suporte ao Candle desativado neste binário (feature flag desabilitada)"
                    .to_string(),
            )
        }
    }

    /// Executa inferência sobre o texto de entrada gerando o embedding vetorial denso.
    pub fn infer(&self, text: &str) -> Result<Vec<f32>, String> {
        #[cfg(feature = "candle")]
        {
            let encoding = self
                .tokenizer
                .encode(text, true)
                .map_err(|e| format!("Erro ao tokenizar: {e}"))?;

            let tokens = encoding.get_ids();
            if tokens.is_empty() {
                return Ok(vec![0.0f32; self.dim]);
            }

            let token_ids = Tensor::new(tokens, &Device::Cpu)
                .map_err(|e| format!("Erro ao criar tensor de tokens: {e}"))?
                .unsqueeze(0)
                .map_err(|e| format!("Erro unsqueeze tokens: {e}"))?;

            let attention_mask_vec: Vec<u32> = encoding.get_attention_mask().to_vec();
            let attention_mask = Tensor::new(attention_mask_vec.as_slice(), &Device::Cpu)
                .map_err(|e| format!("Erro ao criar tensor de attention_mask: {e}"))?
                .unsqueeze(0)
                .map_err(|e| format!("Erro unsqueeze mask: {e}"))?;

            let token_type_ids = token_ids
                .zeros_like()
                .map_err(|e| format!("Erro token_type_ids: {e}"))?;

            let hidden_states = self
                .model
                .forward(&token_ids, &token_type_ids, Some(&attention_mask))
                .map_err(|e| format!("Erro no forward do modelo: {e}"))?;

            // Mean pooling ponderado pela attention_mask:
            // hidden_states: [1, seq_len, dim]
            // attention_mask: [1, seq_len] -> [1, seq_len, 1]
            let mask_f32 = attention_mask
                .to_dtype(DType::F32)
                .map_err(|e| format!("Erro cast mask: {e}"))?
                .unsqueeze(2)
                .map_err(|e| format!("Erro unsqueeze mask: {e}"))?;

            let weighted_states = hidden_states
                .broadcast_mul(&mask_f32)
                .map_err(|e| format!("Erro broadcast mul: {e}"))?;

            let sum_states = weighted_states
                .sum(1)
                .map_err(|e| format!("Erro sum states: {e}"))?
                .squeeze(0)
                .map_err(|e| format!("Erro squeeze sum: {e}"))?;

            let mask_sum = mask_f32
                .sum(1)
                .map_err(|e| format!("Erro sum mask: {e}"))?
                .squeeze(0)
                .map_err(|e| format!("Erro squeeze mask: {e}"))?;

            let mean_pooled = sum_states
                .broadcast_div(
                    &mask_sum
                        .clamp(1e-9, f64::MAX)
                        .map_err(|e| format!("Erro clamp: {e}"))?,
                )
                .map_err(|e| format!("Erro div mean: {e}"))?;

            let mut vec = mean_pooled
                .to_vec1::<f32>()
                .map_err(|e| format!("Erro to_vec1: {e}"))?;

            // Normalização Euclidiana (L2)
            let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
            if norm > 1e-12 {
                for v in &mut vec {
                    *v /= norm;
                }
            }

            Ok(vec)
        }

        #[cfg(not(feature = "candle"))]
        {
            let _ = text;
            Ok(vec![0.0f32; self.dim])
        }
    }
}

impl EmbeddingProvider for CandleEmbeddingProvider {
    fn embed(&self, text: &str) -> Vec<f32> {
        self.infer(text).unwrap_or_else(|_| vec![0.0f32; self.dim])
    }
}
