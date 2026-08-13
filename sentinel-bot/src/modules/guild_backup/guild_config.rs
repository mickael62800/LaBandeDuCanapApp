//! Configuration per-guild du composant `guild-backup-bot`.
//!
//! Lue depuis l'API (table `bot_guild_config`, stockee sous le nom de bot
//! `guild-backup-bot`). Fournit des defauts raisonnables si non configuree.
//! Consommee surtout par le chemin EVENT (pilotage web) pour decider si le
//! composant est actif et appliquer le quota de snapshots.

use std::collections::HashMap;

use crate::shared::api_client::BaseApiClient;

/// Nom sous lequel la config du composant est stockee cote API.
pub const MODULE_BOT_NAME: &str = "guild-backup-bot";

/// Configuration du composant guild-backup pour une guild.
pub struct Config {
    raw: HashMap<String, String>,
}

impl Config {
    /// Charge la config depuis l'API. Config vide (defauts) si l'appel echoue.
    pub async fn load(api: &BaseApiClient, guild_id: &str) -> Self {
        let raw = match api.get_guild_config_for(guild_id, MODULE_BOT_NAME).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %guild_id, "guild_backup: echec get_guild_config");
                HashMap::new()
            }
        };
        Self { raw }
    }

    /// Composant active pour cette guild ? (defaut: false, fail-closed)
    pub fn enabled(&self) -> bool {
        BaseApiClient::config_bool(&self.raw, "enabled", false)
    }

    /// Quota de snapshots conserves (defaut: 10). Les plus anciens au-dela sont
    /// elagues. NB: l'API applique deja son propre quota (20 en dur cote
    /// service) — le plus petit des deux s'applique de fait.
    pub fn snapshot_quota(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "snapshot_quota", 10)
    }

    // ─────────────────────────────────────────────────────────────────────
    // FONCTIONNALITES ANNONCEES MAIS NON IMPLEMENTEES
    //
    // Trois accesseurs vivaient ici derriere un `#![allow(dead_code)]` :
    // `auto_backup_enabled`, `auto_backup_interval_hours` et
    // `restore_role_ids`. Aucun n'etait appele. Ils ont ete supprimes, mais
    // les trois REGLAGES correspondants sont toujours exposes aux
    // administrateurs par `bot_definitions` (cf. migration 001_init.sql,
    // module `guild-backup-bot`) :
    //
    // - « Sauvegarde automatique » + « Intervalle de sauvegarde auto » :
    //   activables dans l'interface, mais aucune capture periodique n'existe.
    // - « Roles autorises a restaurer » : presente comme un controle d'acces,
    //   jamais verifie. `events.rs` le dit explicitement dans un commentaire.
    //   Seule la gate Owner cote API/web protege reellement la restauration.
    //
    // A trancher : implementer, ou retirer ces cles du schema de config pour
    // ne plus promettre ce que le code ne fait pas.
    // ─────────────────────────────────────────────────────────────────────
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(entries: &[(&str, &str)]) -> Config {
        Config {
            raw: entries
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn defaults_when_empty() {
        let c = cfg(&[]);
        assert!(!c.enabled());
        assert_eq!(c.snapshot_quota(), 10);
    }

    #[test]
    fn parses_overrides() {
        let c = cfg(&[("enabled", "false"), ("snapshot_quota", "3")]);
        assert!(!c.enabled());
        assert_eq!(c.snapshot_quota(), 3);
    }
}
