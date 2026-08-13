//! Helpers purs pour parser les valeurs de `bot_guild_config`.
//!
//! Les configs sont stockees en `TEXT` (cle/valeur stringifiees). Ces
//! parsers gardent un defaut si la cle est absente ou si la valeur ne
//! parse pas — defensif. Utilise par les application services taunts
//! (seuils jackpot/donor, flag bankruptcy) et potentiellement d'autres.

use std::collections::HashMap;

/// Les deux flags `enabled` sont la sémantique de référence du dépôt, donc
/// hébergés par le socle : `platform-common/src/config_flags.rs`. Les laisser
/// ici obligeait auparavant le socle worker — et par ricochet les workers Nexus
/// et Atrium — à dépendre du domaine de Sentinel. Ce chemin d'import reste la
/// porte d'entrée pour tout Sentinel ; seule leur adresse a changé.
pub use platform_common::config_flags::{is_worker_service, parse_bool_str, parse_enabled_flag};

/// Parse un flag booleen depuis un map de config. Accepte (insensible a
/// la casse) : `"true"`, `"1"`, `"yes"`. Tout le reste = false.
pub fn parse_bool_config(map: &HashMap<String, String>, key: &str, default: bool) -> bool {
    map.get(key).map(|v| parse_bool_str(v)).unwrap_or(default)
}

/// Parse un entier i64 depuis un map de config. Si la cle est absente
/// ou si la valeur ne parse pas, retourne `default`.
pub fn parse_i64_config(map: &HashMap<String, String>, key: &str, default: i64) -> i64 {
    map.get(key)
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default)
}

/// Parse des lignes "label|value" (separateur pipe).
/// Ignore les lignes vides, sans pipe, ou avec label/value vide.
pub fn parse_pipe_lines(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (left, right) = line.split_once('|')?;
            let left = left.trim();
            let right = right.trim();
            if left.is_empty() || right.is_empty() {
                return None;
            }
            Some((left.to_string(), right.to_string()))
        })
        .collect()
}

/// Parse des lignes "id:value" ou id est un u64 et value est un u64.
pub fn parse_id_u64_lines(raw: &str) -> Vec<(u64, u64)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (id_str, val_str) = line.split_once(':')?;
            let id: u64 = id_str.trim().parse().ok()?;
            let val: u64 = val_str.trim().parse().ok()?;
            Some((id, val))
        })
        .collect()
}

/// Decoupe une chaine CSV en Vec<String> (trim + lowercase, ignore les vides).
pub fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Parse une liste CSV d'entiers u64 (les entrées non numériques sont
/// ignorées).
pub fn parse_u64_csv(raw: &str) -> Vec<u64> {
    raw.split(',')
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .collect()
}

/// Lookup dans un Vec<(u64, u64)> par id.
pub fn lookup_u64(entries: &[(u64, u64)], id: u64) -> Option<u64> {
    entries.iter().find(|(k, _)| *k == id).map(|(_, v)| *v)
}

/// Convention de nommage : les services de type "worker" (jobs batch
/// planifies) ont `worker` dans leur nom. Les autres sont des bots
/// Discord. Utilise par le dashboard pour afficher les compteurs
/// bots_online / workers_online.
/// Categorie de log par defaut d'un service, derivee de son nom :
/// `worker` (jobs batch), `bot` (suffixe `-bot`), sinon `discord`.
pub fn default_log_category(bot_name: &str) -> &'static str {
    if is_worker_service(bot_name) {
        "worker"
    } else if bot_name.contains("-bot") {
        "bot"
    } else {
        "discord"
    }
}

#[cfg(test)]
#[path = "tests/config_parsers.rs"]
mod tests;
