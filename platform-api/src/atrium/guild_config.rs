//! Lecture et écriture de la configuration Atrium par serveur
//! (`bot_guild_config`).
//!
//! Les reglages etaient lus une seule fois au demarrage depuis l'environnement
//! du processus : meme valeur pour tous les serveurs, et un redemarrage
//! necessaire pour changer un plafond. La regle du depot est l'inverse — un
//! reglage se declare dans `bot_definitions.config_schema` et se lit dans
//! `bot_guild_config`, l'environnement ne servant que de repli.
//!
//! Les tables vivent dans la base d'Atrium, pas dans celle de Sentinel :
//! atrium-api n'a aucun acces a `discord_sentinel`. Meme choix que Nexus.

use std::collections::HashMap;

use sqlx::{PgPool, Row};

/// Nom du bot dans `bot_definitions` / `bot_guild_config`.
pub const BOT_NAME: &str = "atrium-bot";

/// Valeurs de repli, issues de l'environnement au demarrage.
///
/// Elles s'appliquent a un serveur qui n'a jamais rien regle. Une installation
/// existante se comporte donc exactement comme avant cette evolution.
#[derive(Debug, Clone, Copy)]
pub struct ConfigDefaults {
    pub user_daily_limit: i32,
    pub user_cooldown_secs: i64,
    pub global_daily_limit: i32,
}

/// Reglages effectifs d'un serveur : valeur configuree, sinon repli.
#[derive(Debug, Clone, Copy)]
pub struct GuildSettings {
    pub enabled: bool,
    pub user_daily_limit: i32,
    pub user_cooldown_secs: i64,
    pub global_daily_limit: i32,
}

/// "true"/"1"/"yes" (insensible a la casse) => true, tout le reste => false.
///
/// Meme semantique que `parse_bool_str` cote Sentinel, qui reste la reference
/// du depot.
fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes"
    )
}

/// Charge toutes les cles configurees pour un serveur.
pub async fn load(pool: &PgPool, guild_id: &str) -> Result<HashMap<String, String>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT config_key, config_value FROM bot_guild_config \
         WHERE guild_id = $1 AND bot_name = $2",
    )
    .bind(guild_id)
    .bind(BOT_NAME)
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::with_capacity(rows.len());
    for row in rows {
        map.insert(
            row.try_get::<String, _>("config_key")?,
            row.try_get::<String, _>("config_value")?,
        );
    }
    Ok(map)
}

/// Etat d'activation deduit d'un instantane brut deja charge.
///
/// L'activation ne depend pas des replis d'environnement (cle absente =
/// DESACTIVE, fail-closed) : cette forme evite de transporter des `defaults`
/// inutiles quand on ne veut que l'etat on/off.
pub fn enabled(raw: &HashMap<String, String>) -> bool {
    raw.get("enabled").map(|v| parse_bool(v)).unwrap_or(false)
}

/// Reglages effectifs, replis appliques.
pub async fn settings(
    pool: &PgPool,
    guild_id: &str,
    defaults: ConfigDefaults,
) -> Result<GuildSettings, sqlx::Error> {
    let raw = load(pool, guild_id).await?;
    Ok(from_map(&raw, defaults))
}

/// Projette une map de config brute sur les reglages effectifs.
///
/// DEFAUT DE `enabled` : cle absente = module DESACTIVE.
///
/// Meme semantique fail-closed que Sentinel (`config_parsers::parse_enabled_flag`),
/// et pour la meme raison : un module ne doit agir sur un serveur que si
/// quelqu'un l'a explicitement active, sinon le tableau de bord affiche
/// « inactif » pendant que le bot repond.
///
/// L'ancien comportement d'`atrium_guild_settings` etait l'inverse
/// (`unwrap_or(true)`). La bascule est sans effet sur l'existant : la migration
/// 007 ecrit une valeur explicite pour tout serveur qu'Atrium a reellement
/// servi, et l'interrupteur du back-office en ecrit toujours une.
pub fn from_map(raw: &HashMap<String, String>, defaults: ConfigDefaults) -> GuildSettings {
    // Generique sur le type cible : le cooldown est un i64 (secondes), les
    // compteurs des i32. Un closure non generique figerait le type au premier
    // appel et ne compilerait pas pour le second.
    fn number<T: std::str::FromStr>(raw: &HashMap<String, String>, key: &str) -> Option<T> {
        raw.get(key).and_then(|v| v.trim().parse().ok())
    }

    GuildSettings {
        enabled: enabled(raw),
        user_daily_limit: number(raw, "user_daily_limit").unwrap_or(defaults.user_daily_limit),
        user_cooldown_secs: number(raw, "user_cooldown_secs")
            .unwrap_or(defaults.user_cooldown_secs),
        global_daily_limit: number(raw, "global_daily_limit")
            .unwrap_or(defaults.global_daily_limit),
    }
}

/// Ecrit (ou remplace) une cle de config pour un serveur.
pub async fn set(pool: &PgPool, guild_id: &str, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (guild_id, bot_name, config_key) \
         DO UPDATE SET config_value = EXCLUDED.config_value, updated_at = now()",
    )
    .bind(guild_id)
    .bind(BOT_NAME)
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULTS: ConfigDefaults = ConfigDefaults {
        user_daily_limit: 30,
        user_cooldown_secs: 10,
        global_daily_limit: 500,
    };

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn config_absente_retombe_sur_l_environnement() {
        let s = from_map(&map(&[]), DEFAULTS);
        // Fail-closed : sans activation explicite, Atrium n'agit pas.
        assert!(
            !s.enabled,
            "un serveur non configure doit rester inactif (fail-closed)"
        );
        assert_eq!(s.user_daily_limit, 30);
        assert_eq!(s.user_cooldown_secs, 10);
        assert_eq!(s.global_daily_limit, 500);
    }

    #[test]
    fn valeurs_configurees_priment() {
        let s = from_map(
            &map(&[
                ("enabled", "false"),
                ("user_daily_limit", "5"),
                ("user_cooldown_secs", "0"),
                ("global_daily_limit", "42"),
            ]),
            DEFAULTS,
        );
        assert!(!s.enabled);
        assert_eq!(s.user_daily_limit, 5);
        assert_eq!(s.user_cooldown_secs, 0);
        assert_eq!(s.global_daily_limit, 42);
    }

    #[test]
    fn valeur_illisible_retombe_sur_le_defaut() {
        // Une saisie corrompue ne doit pas supprimer le plafond : ce serait
        // ouvrir les vannes de la facturation sur une faute de frappe.
        let s = from_map(&map(&[("global_daily_limit", "beaucoup")]), DEFAULTS);
        assert_eq!(s.global_daily_limit, 500);
    }

    #[test]
    fn enabled_libre_suit_la_semantique_fail_closed() {
        // Cle absente = desactive.
        assert!(!enabled(&map(&[])));
        // Meme table de verite que `from_map(...).enabled`, sans defaults.
        for v in ["true", "1", "yes", "TRUE", " Yes "] {
            assert!(enabled(&map(&[("enabled", v)])), "{v}");
        }
        for v in ["false", "0", "no", "n'importe quoi"] {
            assert!(!enabled(&map(&[("enabled", v)])), "{v}");
        }
        // Coherence stricte avec le champ derive par `from_map`.
        for pairs in [vec![], vec![("enabled", "true")], vec![("enabled", "x")]] {
            let m = map(&pairs);
            assert_eq!(enabled(&m), from_map(&m, DEFAULTS).enabled);
        }
    }

    #[test]
    fn semantique_booleenne_de_reference() {
        for v in ["true", "1", "yes", "TRUE", " Yes "] {
            assert!(from_map(&map(&[("enabled", v)]), DEFAULTS).enabled, "{v}");
        }
        for v in ["false", "0", "no", "n'importe quoi"] {
            assert!(!from_map(&map(&[("enabled", v)]), DEFAULTS).enabled, "{v}");
        }
    }
}
