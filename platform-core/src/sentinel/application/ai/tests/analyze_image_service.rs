use super::*;

#[test]
fn test_preprocess_invalid_bytes_returns_error() {
    let result = preprocess_image(&[0, 1, 2, 3]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Image invalide"));
}

#[test]
fn test_preprocess_valid_png() {
    let mut buf = Vec::new();
    {
        use image::ImageBuffer;
        use image::Rgb;
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_, _| Rgb([128, 64, 200]));
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    }
    let tensor = preprocess_image(&buf).unwrap();
    assert_eq!(tensor.shape(), &[1, 3, 224, 224]);
}

#[test]
fn test_preprocess_normalization_range() {
    let mut buf = Vec::new();
    {
        use image::ImageBuffer;
        use image::Rgb;
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(1, 1, |_, _| Rgb([255, 255, 255]));
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    }
    let tensor = preprocess_image(&buf).unwrap();
    let val_r = tensor[[0, 0, 0, 0]];
    assert!((val_r - 2.249).abs() < 0.01);
}

#[test]
fn test_preprocess_black_pixel_normalization() {
    let mut buf = Vec::new();
    {
        use image::ImageBuffer;
        use image::Rgb;
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(1, 1, |_, _| Rgb([0, 0, 0]));
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    }
    let tensor = preprocess_image(&buf).unwrap();
    let val_r = tensor[[0, 0, 0, 0]];
    assert!((val_r - (-2.118)).abs() < 0.01);
}

// ── parse_vision_config (pure helper) ──

use crate::sentinel::domain::entities::system::bot_config::BotGuildConfig;

fn cfg(key: &str, value: &str) -> BotGuildConfig {
    BotGuildConfig {
        id: uuid::Uuid::new_v4(),
        guild_id: "g".into(),
        bot_name: "automod-bot".into(),
        config_key: key.into(),
        config_value: value.into(),
        updated_at: chrono::Utc::now(),
    }
}

#[test]
fn parse_vision_config_defaults_when_no_entries() {
    let c = parse_vision_config(&[]);
    assert!(c.enabled);
    assert!((c.threshold - 0.5).abs() < 1e-6);
}

#[test]
fn parse_vision_config_reads_enabled_truthy_variants() {
    for v in ["true", "1", "yes", "TRUE", "Yes"] {
        let e = parse_vision_config(&[cfg("vision_enabled", v)]).enabled;
        assert!(e, "expected enabled for {v}");
    }
}

#[test]
fn parse_vision_config_enabled_false_for_other_values() {
    for v in ["false", "0", "no", "off", ""] {
        let e = parse_vision_config(&[cfg("vision_enabled", v)]).enabled;
        assert!(!e, "expected disabled for {v}");
    }
}

#[test]
fn parse_vision_config_threshold_clamps_to_0_1() {
    let t1 = parse_vision_config(&[cfg("vision_threshold", "2.5")]).threshold;
    assert_eq!(t1, 1.0);
    let t2 = parse_vision_config(&[cfg("vision_threshold", "-0.3")]).threshold;
    assert_eq!(t2, 0.0);
    let t3 = parse_vision_config(&[cfg("vision_threshold", "0.75")]).threshold;
    assert!((t3 - 0.75).abs() < 1e-6);
}

#[test]
fn parse_vision_config_ignores_invalid_threshold() {
    let t = parse_vision_config(&[cfg("vision_threshold", "not_a_number")]).threshold;
    assert!((t - 0.5).abs() < 1e-6);
}

#[test]
fn parse_vision_config_ignores_unknown_keys() {
    let c = parse_vision_config(&[cfg("some_other_key", "true")]);
    assert!(c.enabled);
    assert!((c.threshold - 0.5).abs() < 1e-6);
}
