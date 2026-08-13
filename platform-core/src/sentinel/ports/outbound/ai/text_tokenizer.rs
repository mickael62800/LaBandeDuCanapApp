//! Port de tokenisation pour les modeles text. L'adapter impl wrap
//! tokenizers::Tokenizer (HuggingFace).

use ndarray::Array2;

pub trait TextTokenizer: Send + Sync {
    fn available(&self) -> bool;
    /// Retourne (input_ids, attention_mask) shape (1, seq_len).
    fn tokenize(&self, text: &str) -> Result<(Array2<i64>, Array2<i64>), String>;
}
