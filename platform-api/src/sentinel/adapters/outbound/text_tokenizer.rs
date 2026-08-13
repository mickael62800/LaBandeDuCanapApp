use std::path::Path;

use ndarray::Array2;
use tokenizers::Tokenizer;
use tracing::info;
use tracing::warn;

pub use platform_core::sentinel::ports::outbound::ai::text_tokenizer::TextTokenizer as TextTokenizerPort;
/// Wrapper autour du tokenizer HuggingFace pour preparer les inputs du modele text ONNX.
pub struct TextTokenizer {
    tokenizer: Option<Tokenizer>,
    max_length: usize,
}

impl TextTokenizer {
    /// Charge un tokenizer depuis un fichier tokenizer.json.
    /// Si le fichier n'existe pas, le tokenizer fonctionne en mode degrade (pas d'inference text).
    pub fn new(tokenizer_path: Option<&str>, max_length: usize) -> Self {
        let tokenizer = tokenizer_path.and_then(|p| {
            if !Path::new(p).exists() {
                warn!(path = %p, "Tokenizer introuvable — inference text desactivee");
                return None;
            }
            match Tokenizer::from_file(p) {
                Ok(mut tok) => {
                    // Detecter le pad token du modele (CamemBERT=<pad>/1, BERT=[PAD]/0)
                    let (pad_id, pad_token) = tok
                        .get_vocab(true)
                        .iter()
                        .find(|(token, _)| *token == "<pad>" || *token == "[PAD]")
                        .map(|(token, &id)| (id, token.clone()))
                        .unwrap_or((0, "[PAD]".to_string()));

                    let padding = tokenizers::PaddingParams {
                        strategy: tokenizers::PaddingStrategy::Fixed(max_length),
                        pad_id,
                        pad_token,
                        ..Default::default()
                    };
                    tok.with_padding(Some(padding));

                    let truncation = tokenizers::TruncationParams {
                        max_length,
                        ..Default::default()
                    };
                    tok.with_truncation(Some(truncation)).ok();

                    info!(path = %p, max_length, "Tokenizer charge");
                    Some(tok)
                }
                Err(e) => {
                    warn!(error = %e, "Erreur chargement tokenizer");
                    None
                }
            }
        });

        Self {
            tokenizer,
            max_length,
        }
    }

    pub fn available(&self) -> bool {
        self.tokenizer.is_some()
    }

    /// Tokenise un texte et retourne (input_ids, attention_mask) prets pour ONNX.
    /// Shape : (1, max_length) pour les deux tensors.
    pub fn tokenize(&self, text: &str) -> Result<(Array2<i64>, Array2<i64>), String> {
        let tokenizer = self.tokenizer.as_ref().ok_or("Tokenizer non charge")?;

        let encoding = tokenizer
            .encode(text, true)
            .map_err(|e| format!("Erreur tokenisation: {e}"))?;

        Ok(build_arrays_from_encoding(
            encoding.get_ids(),
            encoding.get_attention_mask(),
            self.max_length,
        ))
    }
}

impl TextTokenizerPort for TextTokenizer {
    fn available(&self) -> bool {
        TextTokenizer::available(self)
    }
    fn tokenize(&self, text: &str) -> Result<(Array2<i64>, Array2<i64>), String> {
        TextTokenizer::tokenize(self, text)
    }
}

/// Helper pur : convertit les slices ids/mask d'une encoding en Array2<i64>
/// de shape (1, min(len, max_length)). Extrait de `tokenize` pour permettre
/// le test unitaire sans charger un vrai Tokenizer.
pub(super) fn build_arrays_from_encoding(
    ids: &[u32],
    mask: &[u32],
    max_length: usize,
) -> (Array2<i64>, Array2<i64>) {
    let seq_len = ids.len().min(max_length);
    let input_ids = Array2::from_shape_fn((1, seq_len), |(_, j)| ids[j] as i64);
    let attention_mask = Array2::from_shape_fn((1, seq_len), |(_, j)| mask[j] as i64);
    (input_ids, attention_mask)
}

#[cfg(test)]
#[path = "tests/text_tokenizer.rs"]
mod tests;
