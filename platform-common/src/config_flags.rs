//! Sémantique de référence des flags de `bot_guild_config`.
//!
//! # Pourquoi ici plutôt que dans un `-core`
//!
//! Ces deux fonctions décident si un module tourne sur un serveur. Elles ont
//! longtemps vécu dans `sentinel-core`, ce qui obligeait quiconque en avait
//! besoin — auparavant jusque dans le socle des workers des trois plateformes — à
//! dépendre du domaine de Sentinel. Résultat : `nexus-worker` et
//! `atrium-worker` compilaient tout `sentinel-core` pour deux `matches!`.
//!
//! L'alternative (recopier le parsing) est pire : `nexus-core` en a porté une
//! copie au défaut inversé (clé absente = activé), exactement le genre de
//! divergence que la règle d'or 5 du dépôt interdit. Une règle qui doit être
//! identique partout appartient au socle, pas à une plateforme.
//!
//! `platform_core::sentinel::domain::entities::system::config_parsers` les ré-exporte :
//! les appelants Sentinel et la documentation qui les y désigne restent
//! valides.

/// Parse un flag booleen stringifie. Accepte (insensible a la casse) :
/// `"true"`, `"1"`, `"yes"`. Tout le reste = false. Semantique de reference
/// du repo — bot, API et worker doivent tous passer par ici.
pub fn parse_bool_str(v: &str) -> bool {
    matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes")
}

/// Flag d'activation d'un module : ABSENT = DÉSACTIVÉ, présent =
/// `parse_bool_str`. Sémantique unique pour tous les gardes `enabled`
/// per-guild (bot, API, worker) et miroir de `parseBoolConfig` côté web.
///
/// Fail-closed : un module n'agit sur un serveur que si quelqu'un l'a
/// explicitement activé. Avant, l'absence de ligne valait « actif », ce qui
/// faisait tourner des modules que le dashboard présentait comme inactifs.
/// Conséquence assumée : après ce changement, chaque module doit être activé
/// depuis la page Composants pour reprendre du service.
pub fn parse_enabled_flag(value: Option<&str>) -> bool {
    value.map(parse_bool_str).unwrap_or(false)
}

/// Identifie les anciens services batch à partir de leur convention de nom.
/// Cette classification est partagée par la supervision et le domaine
/// Sentinel ; elle n'appartient donc à aucune entité fonctionnelle.
pub fn is_worker_service(name: &str) -> bool {
    name.contains("worker")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bool_str_accepte_les_trois_formes() {
        for v in ["true", "TRUE", "1", "yes", "YeS"] {
            assert!(parse_bool_str(v), "{v} devrait valoir true");
        }
        for v in ["false", "0", "no", "", "oui"] {
            assert!(!parse_bool_str(v), "{v} devrait valoir false");
        }
    }

    /// Le point le plus important du module : une clé absente ne doit JAMAIS
    /// valoir « activé ». C'est ce que la copie de `nexus-core` avait inversé.
    #[test]
    fn cle_absente_vaut_desactive() {
        assert!(!parse_enabled_flag(None));
        assert!(!parse_enabled_flag(Some("false")));
        assert!(parse_enabled_flag(Some("true")));
    }
}
