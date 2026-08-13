use super::*;

#[test]
fn test_tokenizer_none_path_not_available() {
    let tok = TextTokenizer::new(None, 256);
    assert!(!tok.available());
}

#[test]
fn test_tokenizer_nonexistent_path_not_available() {
    let tok = TextTokenizer::new(Some("/nonexistent/tokenizer.json"), 256);
    assert!(!tok.available());
}

#[test]
fn test_tokenize_without_tokenizer_returns_error() {
    let tok = TextTokenizer::new(None, 256);
    let result = tok.tokenize("hello world");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("non charge"));
}

// ── Tests du helper pur `build_arrays_from_encoding` ──

#[test]
fn build_arrays_short_sequence_truncated_to_len() {
    let (ids, mask) = build_arrays_from_encoding(&[1, 2, 3], &[1, 1, 1], 256);
    // seq_len = min(3, 256) = 3
    assert_eq!(ids.shape(), &[1, 3]);
    assert_eq!(mask.shape(), &[1, 3]);
    assert_eq!(ids[[0, 0]], 1);
    assert_eq!(ids[[0, 2]], 3);
    assert_eq!(mask[[0, 0]], 1);
}

#[test]
fn build_arrays_long_sequence_capped_to_max_length() {
    let ids: Vec<u32> = (0..500).collect();
    let mask: Vec<u32> = vec![1; 500];
    let (out_ids, out_mask) = build_arrays_from_encoding(&ids, &mask, 256);
    assert_eq!(out_ids.shape(), &[1, 256]);
    assert_eq!(out_mask.shape(), &[1, 256]);
    assert_eq!(out_ids[[0, 0]], 0);
    assert_eq!(out_ids[[0, 255]], 255);
}

#[test]
fn build_arrays_empty_sequence() {
    let (ids, mask) = build_arrays_from_encoding(&[], &[], 256);
    assert_eq!(ids.shape(), &[1, 0]);
    assert_eq!(mask.shape(), &[1, 0]);
}

#[test]
fn build_arrays_u32_max_cast_to_i64() {
    let (ids, _) = build_arrays_from_encoding(&[u32::MAX], &[1], 256);
    assert_eq!(ids[[0, 0]], u32::MAX as i64);
}

#[test]
fn test_max_length_stored() {
    let tok = TextTokenizer::new(None, 128);
    assert_eq!(tok.max_length, 128);
}

// ── Tests avec le vrai tokenizer ──

const TOKENIZER_PATH: &str = "../../platform-ml/text/exports/tokenizer.json";

fn load_real_tokenizer() -> Option<TextTokenizer> {
    let tok = TextTokenizer::new(Some(TOKENIZER_PATH), 256);
    if tok.available() {
        Some(tok)
    } else {
        None
    }
}

#[test]
#[ignore = "Necessite le fichier tokenizer sur le disque"]
fn real_tokenizer_loads_successfully() {
    let tok = load_real_tokenizer();
    assert!(tok.is_some(), "Tokenizer introuvable a {TOKENIZER_PATH}");
}

#[test]
fn real_tokenizer_simple_text() {
    let Some(tok) = load_real_tokenizer() else {
        return;
    };
    let (ids, mask) = tok.tokenize("Bonjour tout le monde").unwrap();
    assert_eq!(ids.shape(), &[1, 256]);
    assert_eq!(mask.shape(), &[1, 256]);
    assert_ne!(ids[[0, 0]], 0);
    assert_eq!(mask[[0, 0]], 1);
    assert_eq!(mask[[0, 255]], 0);
}

#[test]
fn real_tokenizer_empty_text() {
    let Some(tok) = load_real_tokenizer() else {
        return;
    };
    let (ids, mask) = tok.tokenize("").unwrap();
    assert_eq!(ids.shape(), &[1, 256]);
    assert_eq!(mask[[0, 0]], 1);
    let _ = ids;
}

#[test]
fn real_tokenizer_long_text_truncated() {
    let Some(tok) = load_real_tokenizer() else {
        return;
    };
    let long_text = "mot ".repeat(1000);
    let (ids, mask) = tok.tokenize(&long_text).unwrap();
    assert_eq!(ids.shape(), &[1, 256]);
    assert_eq!(mask[[0, 255]], 1);
}

#[test]
fn real_tokenizer_special_chars() {
    let Some(tok) = load_real_tokenizer() else {
        return;
    };
    let result = tok.tokenize("😡🤬💀 je vais te 💩 espèce de $#@!");
    assert!(result.is_ok());
}

#[test]
fn real_tokenizer_french_insults() {
    let Some(tok) = load_real_tokenizer() else {
        return;
    };
    let result = tok.tokenize("t'es qu'un connard, ferme ta gueule");
    assert!(result.is_ok());
    let (ids, _) = result.unwrap();
    let non_zero = ids.iter().filter(|&&v| v != 0).count();
    assert!(non_zero > 3);
}

// ── Tests avec un tokenizer minimal écrit dans un fichier temporaire ──
//
// Ces tests écrivent un tokenizer.json WordLevel minimal en /tmp pour
// exercer le chemin "tokenizer charge correctement" sans avoir besoin
// du gros modele HuggingFace.

fn write_minimal_tokenizer_json(pad_token: &str, pad_id: u32) -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "sentinel_test_tokenizer_{}_{}.json",
        pad_token
            .replace('<', "a")
            .replace('>', "b")
            .replace('[', "c")
            .replace(']', "d"),
        uuid::Uuid::new_v4().as_u128() % 1_000_000
    ));
    // WordLevel minimal qui se decode avec le crate `tokenizers`.
    let json = serde_json::json!({
        "version": "1.0",
        "truncation": null,
        "padding": null,
        "added_tokens": [
            {"id": pad_id, "content": pad_token, "single_word": false, "lstrip": false, "rstrip": false, "normalized": false, "special": true},
            {"id": 99, "content": "[UNK]", "single_word": false, "lstrip": false, "rstrip": false, "normalized": false, "special": true}
        ],
        "normalizer": null,
        "pre_tokenizer": {"type": "Whitespace"},
        "post_processor": null,
        "decoder": null,
        "model": {
            "type": "WordLevel",
            "vocab": {
                "[UNK]": 99_i32,
                pad_token: pad_id as i32,
                "hello": 2,
                "world": 3,
                "bonjour": 4,
                "monde": 5
            },
            "unk_token": "[UNK]"
        }
    });
    std::fs::write(&path, serde_json::to_string(&json).unwrap()).expect("ecriture tokenizer temp");
    path
}

#[test]
fn minimal_tokenizer_loads_with_pad_detection() {
    let path = write_minimal_tokenizer_json("<pad>", 1);
    let tok = TextTokenizer::new(Some(path.to_str().unwrap()), 16);
    assert!(tok.available());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn minimal_tokenizer_bracket_pad_variant() {
    // Teste la branche `[PAD]` du detector pad token.
    let path = write_minimal_tokenizer_json("[PAD]", 0);
    let tok = TextTokenizer::new(Some(path.to_str().unwrap()), 16);
    assert!(tok.available());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn minimal_tokenizer_tokenize_returns_padded_shape() {
    let path = write_minimal_tokenizer_json("<pad>", 1);
    let tok = TextTokenizer::new(Some(path.to_str().unwrap()), 16);
    let (ids, mask) = tok.tokenize("hello world").unwrap();
    // Padding fixe a 16 → shape (1, 16)
    assert_eq!(ids.shape(), &[1, 16]);
    assert_eq!(mask.shape(), &[1, 16]);
    // Premier token = "hello" = id 2
    assert_eq!(ids[[0, 0]], 2);
    assert_eq!(ids[[0, 1]], 3);
    // Padding apres : ids = pad_id (1), mask = 0
    assert_eq!(ids[[0, 15]], 1);
    assert_eq!(mask[[0, 15]], 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn minimal_tokenizer_handles_unknown_words() {
    let path = write_minimal_tokenizer_json("<pad>", 1);
    let tok = TextTokenizer::new(Some(path.to_str().unwrap()), 16);
    let (ids, _mask) = tok.tokenize("wibble wobble").unwrap();
    // Mots inconnus → UNK token (id 99)
    assert_eq!(ids[[0, 0]], 99);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn malformed_tokenizer_file_falls_back_to_unavailable() {
    // Ecrit du JSON invalide pour un tokenizer
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "sentinel_malformed_{}.json",
        uuid::Uuid::new_v4().as_u128() % 1_000_000
    ));
    std::fs::write(&path, "{not: valid: json}").unwrap();
    let tok = TextTokenizer::new(Some(path.to_str().unwrap()), 16);
    // Erreur de parsing → tokenizer = None
    assert!(!tok.available());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn tokenizer_without_pad_token_uses_default_fallback() {
    // Tokenizer valide mais sans <pad> ni [PAD] → fallback (0, "[PAD]")
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "sentinel_no_pad_{}.json",
        uuid::Uuid::new_v4().as_u128() % 1_000_000
    ));
    let json = serde_json::json!({
        "version": "1.0",
        "truncation": null,
        "padding": null,
        "added_tokens": [
            {"id": 99, "content": "[UNK]", "single_word": false, "lstrip": false, "rstrip": false, "normalized": false, "special": true}
        ],
        "normalizer": null,
        "pre_tokenizer": {"type": "Whitespace"},
        "post_processor": null,
        "decoder": null,
        "model": {
            "type": "WordLevel",
            "vocab": {
                "[UNK]": 99_i32,
                "hello": 2_i32
            },
            "unk_token": "[UNK]"
        }
    });
    std::fs::write(&path, serde_json::to_string(&json).unwrap()).unwrap();
    let tok = TextTokenizer::new(Some(path.to_str().unwrap()), 8);
    // Pas de pad token trouve → fallback (0, "[PAD]") utilise en interne.
    assert!(tok.available());
    // tokenize fonctionne avec padding au defaut
    let (ids, mask) = tok.tokenize("hello").unwrap();
    assert_eq!(ids.shape(), &[1, 8]);
    // Padding ids = 0 (fallback), mask = 0
    assert_eq!(ids[[0, 7]], 0);
    assert_eq!(mask[[0, 7]], 0);
    let _ = std::fs::remove_file(&path);
}
