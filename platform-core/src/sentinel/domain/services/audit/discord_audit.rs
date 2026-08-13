//! Helpers purs pour la synchronisation des audit logs Discord
//! (consommés par sentinel-worker, domaine `discord_audit_sync`).

/// Compare deux snowflakes Discord (stockes en String dans le JSON) par leur
/// valeur u64. Retourne true si `candidate` est plus recent que `current`.
///
/// Historiquement le code faisait une comparaison de Strings, ce qui
/// fonctionne tant que les deux snowflakes ont la **meme longueur** (cas
/// habituel en 2024+ : ~19 digits). Mais c'est un bug latent : si les
/// longueurs different (ex : vieux snowflake 17 digits vs nouveau 19 digits),
/// la comparaison ASCII renvoie un ordre faux et le curseur `last_entry_id`
/// peut se retrouver bloque ou rater des entries.
pub fn is_newer_snowflake(current: Option<&str>, candidate: &str) -> bool {
    let candidate_id: u64 = match candidate.parse() {
        Ok(v) => v,
        Err(_) => return false, // candidate invalide : ne pas avancer le curseur
    };
    match current {
        None => true,
        Some(s) => match s.parse::<u64>() {
            Ok(curr) => candidate_id > curr,
            Err(_) => true, // current invalide : accepter le candidate
        },
    }
}

/// Epoch Discord en millisecondes (2015-01-01T00:00:00Z).
const DISCORD_EPOCH_MILLIS: u64 = 1_420_070_400_000;

/// Retrouve l'heure de creation encodee dans un snowflake Discord.
pub fn snowflake_created_at(snowflake: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let snowflake = snowflake.parse::<u64>().ok()?;
    let millis = (snowflake >> 22).checked_add(DISCORD_EPOCH_MILLIS)?;
    chrono::DateTime::from_timestamp_millis(i64::try_from(millis).ok()?)
}

/// Mapping des action_types Discord numeriques vers des `event_type` lisibles
/// stockes dans `audit_logs`. Les valeurs proviennent de la doc Discord :
/// <https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object-audit-log-events>
///
/// MVP : on couvre uniquement les actions de moderation pertinentes. Les autres
/// (channel/role create, message delete, etc.) retournent None et sont skip.
pub fn map_action_type(action_type: u32) -> Option<String> {
    let name = match action_type {
        20 => "member_kick",
        22 => "member_ban",
        23 => "member_unban",
        24 => "member_timeout",
        25 => "member_role_update",
        _ => return None,
    };
    Some(format!("discord_audit:{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_action_type_member_ban() {
        assert_eq!(
            map_action_type(22),
            Some("discord_audit:member_ban".to_string())
        );
    }

    #[test]
    fn map_action_type_member_unban() {
        assert_eq!(
            map_action_type(23),
            Some("discord_audit:member_unban".to_string())
        );
    }

    #[test]
    fn map_action_type_member_kick() {
        assert_eq!(
            map_action_type(20),
            Some("discord_audit:member_kick".to_string())
        );
    }

    #[test]
    fn map_action_type_member_timeout() {
        assert_eq!(
            map_action_type(24),
            Some("discord_audit:member_timeout".to_string())
        );
    }

    #[test]
    fn map_action_type_member_role_update() {
        assert_eq!(
            map_action_type(25),
            Some("discord_audit:member_role_update".to_string())
        );
    }

    #[test]
    fn map_action_type_unknown_returns_none() {
        assert_eq!(map_action_type(1), None);
        assert_eq!(map_action_type(72), None); // MESSAGE_DELETE pas couvert MVP
        assert_eq!(map_action_type(999), None);
    }

    // ── Tests du helper is_newer_snowflake ────────────────

    #[test]
    fn snowflake_none_current_accepts_anything() {
        assert!(is_newer_snowflake(None, "1234567890123456789"));
    }

    #[test]
    fn snowflake_strictly_greater() {
        assert!(is_newer_snowflake(
            Some("1234567890123456789"),
            "1234567890123456790"
        ));
    }

    #[test]
    fn snowflake_equal_is_not_newer() {
        assert!(!is_newer_snowflake(
            Some("1234567890123456789"),
            "1234567890123456789"
        ));
    }

    #[test]
    fn snowflake_strictly_smaller() {
        assert!(!is_newer_snowflake(
            Some("1234567890123456790"),
            "1234567890123456789"
        ));
    }

    /// Regression test pour le bug P0 : comparaison string vs u64.
    /// En string, "2" < "10" serait false car '2' > '1' en ASCII.
    /// En u64, 2 < 10 est vrai.
    #[test]
    fn snowflake_different_lengths_compared_numerically() {
        // "2" (1 digit) < "10" (2 digits) en u64 -> 10 doit etre newer que 2
        assert!(is_newer_snowflake(Some("2"), "10"));

        // "99" (2 digits) < "100" (3 digits) en u64 mais "99" > "100" en string
        assert!(is_newer_snowflake(Some("99"), "100"));

        // Sens inverse : "1000" > "999" en u64
        assert!(!is_newer_snowflake(Some("1000"), "999"));
    }

    #[test]
    fn snowflake_invalid_candidate_ignored() {
        assert!(!is_newer_snowflake(Some("1234"), "not-a-number"));
    }

    #[test]
    fn snowflake_invalid_current_accepts_candidate() {
        // Si le curseur en base est corrompu (pas un u64), on accepte le
        // candidate pour se re-synchroniser.
        assert!(is_newer_snowflake(Some("corrupted"), "1234567890123456789"));
    }

    #[test]
    fn snowflake_timestamp_uses_discord_epoch() {
        let expected_millis = 1_700_000_000_000u64;
        let snowflake = ((expected_millis - DISCORD_EPOCH_MILLIS) << 22).to_string();

        let created_at = snowflake_created_at(&snowflake).unwrap();

        assert_eq!(created_at.timestamp_millis(), expected_millis as i64);
    }

    #[test]
    fn invalid_snowflake_has_no_timestamp() {
        assert!(snowflake_created_at("invalid").is_none());
    }
}
