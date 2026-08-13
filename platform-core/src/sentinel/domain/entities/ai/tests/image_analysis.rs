use super::*;

#[test]
fn max_image_size_is_14mb_base64() {
    assert_eq!(MAX_IMAGE_BASE64_LEN, 14_000_000);
}

#[test]
fn allowed_types_cover_common_formats() {
    assert!(ALLOWED_IMAGE_CONTENT_TYPES.contains(&"image/jpeg"));
    assert!(ALLOWED_IMAGE_CONTENT_TYPES.contains(&"image/png"));
    assert!(ALLOWED_IMAGE_CONTENT_TYPES.contains(&"image/gif"));
    assert!(ALLOWED_IMAGE_CONTENT_TYPES.contains(&"image/webp"));
    assert!(ALLOWED_IMAGE_CONTENT_TYPES.contains(&"image/bmp"));
    assert_eq!(ALLOWED_IMAGE_CONTENT_TYPES.len(), 5);
}

#[test]
fn is_allowed_type_exact() {
    assert!(is_allowed_image_content_type("image/jpeg"));
    assert!(is_allowed_image_content_type("image/png"));
    assert!(!is_allowed_image_content_type("image/svg+xml"));
    assert!(!is_allowed_image_content_type("application/pdf"));
    assert!(!is_allowed_image_content_type(""));
}

#[test]
fn is_allowed_type_case_insensitive() {
    assert!(is_allowed_image_content_type("IMAGE/JPEG"));
    assert!(is_allowed_image_content_type("Image/Png"));
}

#[test]
fn is_size_acceptable_boundaries() {
    assert!(is_image_size_acceptable(0));
    assert!(is_image_size_acceptable(1));
    assert!(is_image_size_acceptable(MAX_IMAGE_BASE64_LEN));
    assert!(!is_image_size_acceptable(MAX_IMAGE_BASE64_LEN + 1));
    assert!(!is_image_size_acceptable(usize::MAX));
}
