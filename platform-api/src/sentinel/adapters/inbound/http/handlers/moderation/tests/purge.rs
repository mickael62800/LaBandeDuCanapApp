use super::*;

#[test]
fn purge_by_days_dto_deserializes() {
    let raw = r#"{"guild_id":"g","days":30}"#;
    let dto: PurgeByDaysDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.days, 30);
}

#[test]
fn purge_by_days_dto_zero_days_allowed() {
    // days=0 autorise pour infractions (signifie "tout supprimer")
    let dto: PurgeByDaysDto = serde_json::from_str(r#"{"guild_id":"g","days":0}"#).unwrap();
    assert_eq!(dto.days, 0);
}

#[test]
fn purge_by_days_dto_negative_days_deserializes() {
    // Le deserialize accepte les negatifs, la validation est faite par le handler.
    let dto: PurgeByDaysDto = serde_json::from_str(r#"{"guild_id":"g","days":-5}"#).unwrap();
    assert_eq!(dto.days, -5);
}

#[test]
fn purge_logs_dto_deserializes() {
    let dto: PurgeLogsDto = serde_json::from_str(r#"{"days":90}"#).unwrap();
    assert_eq!(dto.days, 90);
}

#[test]
fn purge_logs_dto_large_days_allowed() {
    let dto: PurgeLogsDto = serde_json::from_str(r#"{"days":3650}"#).unwrap();
    assert_eq!(dto.days, 3650);
}
