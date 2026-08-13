//! Game Server (instance) — represente un container Docker piloté.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Etats possibles d'un serveur. Doit rester synchronise avec le CHECK
/// constraint en migration (188_game_portal_init.sql).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameServerStatus {
    /// Ligne creee, container pas encore lance.
    Created,
    /// Ouverture programmee : le conteneur n'est PAS lance, mais les salons
    /// Discord et le panneau d'inscription existent deja (les joueurs peuvent
    /// s'inscrire a l'avance). Le worker demarre le conteneur ~5 min avant
    /// `ip_reveal_at`, puis l'IP est revelee a l'heure dite.
    Scheduled,
    /// Docker start envoye, attente health.
    Starting,
    /// Container running, repond aux health checks.
    Running,
    /// Docker stop envoye, attente fin du process.
    Stopping,
    /// Container arrete proprement.
    Stopped,
    /// Crash repete ou erreur de boot, plus auto-restart.
    Error,
    /// Soft-deleted (volume + container retires). Conserve pour audit.
    Deleted,
}

impl GameServerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Scheduled => "scheduled",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Error => "error",
            Self::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "created" => Some(Self::Created),
            "scheduled" => Some(Self::Scheduled),
            "starting" => Some(Self::Starting),
            "running" => Some(Self::Running),
            "stopping" => Some(Self::Stopping),
            "stopped" => Some(Self::Stopped),
            "error" => Some(Self::Error),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }

    /// Etats considere's "actifs" (consomment des ressources).
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Stopping)
    }

    /// True si une transition `start` est legale depuis cet etat. `Scheduled`
    /// en fait partie : c'est le worker (auto-start ~5 min avant l'ouverture)
    /// ou un administrateur qui lance alors le conteneur.
    pub fn can_start(&self) -> bool {
        matches!(
            self,
            Self::Created | Self::Scheduled | Self::Stopped | Self::Error
        )
    }

    /// True si une transition `stop` est legale.
    pub fn can_stop(&self) -> bool {
        matches!(self, Self::Running | Self::Starting)
    }
}

/// Instance persistée d'un template de jeu pour une guilde.
///
/// `GameServer` est la source métier de l'état du portail. Les champs Docker
/// sont des références techniques, tandis que `status`, les dates et les
/// compteurs servent à décider si une transition est autorisée.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameServer {
    /// Identifiant stable de l'instance.
    pub id: Uuid,
    /// Guilde Discord propriétaire de l'instance.
    pub guild_id: String,
    /// Template utilisé pour créer l'instance.
    pub template_id: Uuid,
    /// Nom visible par les administrateurs et les joueurs.
    pub name: String,
    /// État courant et unique du cycle de vie.
    pub status: GameServerStatus,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub host_port: Option<u16>,
    pub rcon_port: Option<u16>,
    pub rcon_password: Option<String>,
    pub volume_name: Option<String>,
    pub allocated_memory_mb: i32,
    /// Plafond CPU en coeurs. None = defaut de l'adapter.
    pub cpu_limit: Option<f64>,
    /// Administrateur ou membre à l'origine de la création.
    pub owner_user_id: String,
    pub idle_shutdown_days: Option<i32>,
    pub last_active_at: Option<DateTime<Utc>>,
    /// Dernier nombre de joueurs connu par le health-check.
    pub last_player_count: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    /// Nombre de redemarrages auto consecutifs apres crash (remis a 0 a la
    /// recuperation). Borne par la config `max_auto_restart_attempts`.
    pub restart_attempts: i32,
    /// Timestamp du dernier redemarrage auto tente (pour le backoff).
    pub last_restart_at: Option<DateTime<Utc>>,
    // ── Session Discord (evenement de serveur) ──
    /// Salon textuel prive cree pour cette session (None si pas cree).
    pub text_channel_id: Option<String>,
    /// Salon vocal prive cree pour cette session.
    pub voice_channel_id: Option<String>,
    /// Date de revelation de l'IP (None = pas de revelation programmee).
    pub ip_reveal_at: Option<DateTime<Utc>>,
    /// True une fois l'IP revelee dans le salon.
    pub ip_revealed: bool,
}

/// Nombre maximal PAR DEFAUT de redemarrages auto consecutifs avant abandon
/// (-> Error). Aligne sur le default du schema game-portal affiche au
/// dashboard (`max_auto_restart_attempts`). La valeur effective est
/// configurable par guild ; ce const n'est que le fallback quand la config
/// est absente. Borne stricte : empeche tout crash loop.
pub const DEFAULT_MAX_RESTART_ATTEMPTS: i32 = 3;

/// Base PAR DEFAUT du backoff (secondes). Aligne sur le default du schema
/// game-portal `restart_backoff_base_secs`. Fallback quand la config est
/// absente.
pub const DEFAULT_RESTART_BACKOFF_BASE_SECS: i64 = 30;

/// Plafond PAR DEFAUT du backoff (secondes = 1h). Aligne sur le default du
/// schema game-portal `restart_backoff_cap_secs`. Fallback quand la config
/// est absente.
pub const DEFAULT_RESTART_BACKOFF_CAP_SECS: i64 = 3600;

/// Delai de backoff (secondes) avant le prochain redemarrage auto, en
/// fonction du nombre de tentatives deja effectuees. Exponentiel
/// `base * 2^attempts`, plafonne a `cap`. `base` et `cap` sont fournis par la
/// couche application (config per-guild) pour garder cette fonction pure /
/// testable unitairement. Pure / overflow-safe.
pub fn restart_backoff_secs(attempts: i32, base_secs: i64, cap_secs: i64) -> i64 {
    if attempts <= 0 {
        return base_secs.min(cap_secs);
    }
    // 2^attempts overflow-safe : checked_shl renvoie None si shift trop grand.
    let factor = 1i64.checked_shl(attempts as u32).unwrap_or(i64::MAX);
    base_secs.saturating_mul(factor).min(cap_secs)
}

/// Decision PURE : faut-il auto-redemarrer un serveur crashe ? `true` si
/// l'auto-restart est active ET on est sous le plafond de tentatives ET
/// (jamais redemarre OU le cooldown de backoff est ecoule). `max_attempts` et
/// `auto_restart_on_crash` ainsi que les parametres de backoff (`base`/`cap`)
/// sont fournis par la couche application (config per-guild) pour garder cette
/// fonction pure / testable unitairement.
#[allow(clippy::too_many_arguments)]
pub fn should_auto_restart(
    auto_restart_on_crash: bool,
    attempts: i32,
    max_attempts: i32,
    last_restart_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    backoff_base_secs: i64,
    backoff_cap_secs: i64,
) -> bool {
    if !auto_restart_on_crash {
        return false;
    }
    if attempts >= max_attempts {
        return false;
    }
    match last_restart_at {
        None => true,
        Some(last) => {
            now >= last
                + chrono::Duration::seconds(restart_backoff_secs(
                    attempts,
                    backoff_base_secs,
                    backoff_cap_secs,
                ))
        }
    }
}

/// Delai FIXE (minutes) entre le demarrage automatique du conteneur d'un
/// serveur `Scheduled` et l'heure de revelation de l'IP. On lance le jeu en
/// avance pour qu'il ait fini de booter (chargement du monde, plugins) au
/// moment ou l'adresse est communiquee aux joueurs.
pub const PREP_LEAD_MINUTES: i64 = 5;

/// Decision PURE : un serveur programme doit-il demarrer maintenant ? `true`
/// des que l'on est a moins de `PREP_LEAD_MINUTES` de l'heure de revelation.
/// `None` (pas d'heure programmee) ne declenche jamais d'auto-start.
pub fn should_auto_start_scheduled(
    ip_reveal_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    match ip_reveal_at {
        Some(reveal_at) => now >= reveal_at - chrono::Duration::minutes(PREP_LEAD_MINUTES),
        None => false,
    }
}

impl GameServer {
    /// Nom Docker normalise pour ce serveur.
    /// Doit matcher container_name persiste en DB (cohérence reconciler).
    ///
    /// # Le prefixe `sentinel-` ne se renomme PAS
    ///
    /// Les jeux ont quitte Sentinel au portage, et les LABELS ont ete
    /// renommes en `nexus.*` (avec lecture des deux generations). Ce nom-ci,
    /// non : il est persiste en base a la creation, et le reconciler compare
    /// la valeur stockee a celle recalculee ici. Changer la formule ferait
    /// diverger les deux pour toute la flotte existante.
    ///
    /// Le renommer supposerait une migration qui recree chaque conteneur —
    /// c'est-a-dire une interruption de tous les serveurs de jeu, pour un gain
    /// purement cosmetique.
    pub fn docker_container_name(id: Uuid) -> String {
        format!("sentinel-game-{}", id.simple())
    }

    /// Nom du volume Docker nomme pour ce serveur.
    ///
    /// # Celui-ci encore moins que l'autre
    ///
    /// Docker ne sait pas renommer un volume. Changer cette formule ne
    /// renommerait rien : au prochain demarrage, le conteneur monterait un
    /// volume NEUF et VIDE, et le monde de jeu apparaitrait efface. L'ancien
    /// volume resterait sur le disque, orphelin.
    ///
    /// Si ce nom doit changer un jour, c'est par une migration de donnees
    /// explicite (creation du nouveau volume, copie, bascule, suppression de
    /// l'ancien), jamais par une edition de cette ligne.
    pub fn docker_volume_name(id: Uuid) -> String {
        format!("sentinel-game-vol-{}", id.simple())
    }
}

/// Commande pour creer un serveur (input du use case).
#[derive(Debug, Clone)]
pub struct CreateGameServerCommand {
    pub guild_id: String,
    pub template_slug: String,
    pub name: String,
    pub allocated_memory_mb: Option<i32>,
    /// Plafond CPU en coeurs (ex: 2.0). None = defaut.
    pub cpu_limit: Option<f64>,
    pub owner_user_id: String,
    pub initial_config: std::collections::HashMap<String, String>,
}

/// Validation regex-like du nom de serveur (alphanumerique + espaces / _ / -).
pub fn validate_server_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err("nom invalide : 1-64 caracteres".into());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_' || c == '-')
    {
        return Err("nom invalide : alphanumerique + espaces, _ ou - uniquement".into());
    }
    Ok(())
}

/// Memoire a accorder au CONTENEUR pour un jeu dote de `heap_mb` de memoire.
///
/// Le conteneur doit toujours en avoir PLUS que le jeu. Une JVM configuree
/// avec 2 Go de tas consomme davantage : metaespace, piles de threads,
/// cache de code, structures du ramasse-miettes, tampons directs. Lui donner
/// exactement son tas la fait tuer par le noyau des le demarrage — c'est
/// silencieux du cote de Docker, le journal du jeu s'arrete simplement sur un
/// code de sortie -1.
///
/// La marge vaut un quart du tas, avec un plancher de 512 Mo : en proportion
/// seule, un petit serveur n'aurait pas de quoi couvrir les couts fixes de la
/// machine virtuelle, qui ne diminuent pas avec le tas.
///
/// S'applique aussi aux jeux natifs : la marge y sert de coussin plutot que
/// de laisser le noyau arbitrer au premier pic.
pub fn container_memory_mb(heap_mb: i32) -> i32 {
    const MARGE_MINIMALE_MB: i32 = 512;
    let marge = (heap_mb / 4).max(MARGE_MINIMALE_MB);
    heap_mb.saturating_add(marge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_start_only_from_terminal_states() {
        for st in [
            GameServerStatus::Created,
            GameServerStatus::Scheduled,
            GameServerStatus::Stopped,
            GameServerStatus::Error,
        ] {
            assert!(st.can_start(), "{st:?} devrait permettre start");
        }
        for st in [
            GameServerStatus::Starting,
            GameServerStatus::Running,
            GameServerStatus::Stopping,
            GameServerStatus::Deleted,
        ] {
            assert!(!st.can_start(), "{st:?} ne devrait pas permettre start");
        }
    }

    #[test]
    fn can_stop_only_from_active_up_states() {
        assert!(GameServerStatus::Running.can_stop());
        assert!(GameServerStatus::Starting.can_stop());
        for st in [
            GameServerStatus::Created,
            GameServerStatus::Stopped,
            GameServerStatus::Error,
            GameServerStatus::Stopping,
            GameServerStatus::Deleted,
        ] {
            assert!(!st.can_stop(), "{st:?} ne devrait pas permettre stop");
        }
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        const B: i64 = DEFAULT_RESTART_BACKOFF_BASE_SECS;
        const C: i64 = DEFAULT_RESTART_BACKOFF_CAP_SECS;
        assert_eq!(restart_backoff_secs(0, B, C), 30);
        assert_eq!(restart_backoff_secs(1, B, C), 60);
        assert_eq!(restart_backoff_secs(2, B, C), 120);
        assert_eq!(restart_backoff_secs(3, B, C), 240);
        // Plafond a 1h, jamais d'overflow meme pour de grandes valeurs.
        assert_eq!(restart_backoff_secs(20, B, C), 3600);
        assert_eq!(restart_backoff_secs(1000, B, C), 3600);
        assert_eq!(restart_backoff_secs(-1, B, C), 30);
    }

    #[test]
    fn backoff_respects_custom_base_and_cap() {
        // base 10s, cap 100s : 10, 20, 40, 80, puis plafonne a 100.
        assert_eq!(restart_backoff_secs(0, 10, 100), 10);
        assert_eq!(restart_backoff_secs(1, 10, 100), 20);
        assert_eq!(restart_backoff_secs(3, 10, 100), 80);
        assert_eq!(restart_backoff_secs(4, 10, 100), 100);
        // base > cap : borne des attempts=0.
        assert_eq!(restart_backoff_secs(0, 200, 100), 100);
    }

    #[test]
    fn should_restart_first_attempt_no_history() {
        let now = Utc::now();
        // Jamais redemarre -> autorise immediatement.
        assert!(should_auto_restart(
            true,
            0,
            DEFAULT_MAX_RESTART_ATTEMPTS,
            None,
            now,
            DEFAULT_RESTART_BACKOFF_BASE_SECS,
            DEFAULT_RESTART_BACKOFF_CAP_SECS,
        ));
    }

    #[test]
    fn should_restart_when_cooldown_elapsed() {
        let now = Utc::now();
        // 2 tentatives -> backoff 120s ; dernier restart il y a 200s -> ok.
        let last = now - chrono::Duration::seconds(200);
        assert!(should_auto_restart(
            true,
            2,
            5,
            Some(last),
            now,
            DEFAULT_RESTART_BACKOFF_BASE_SECS,
            DEFAULT_RESTART_BACKOFF_CAP_SECS,
        ));
    }

    #[test]
    fn should_not_restart_within_cooldown() {
        let now = Utc::now();
        // 2 tentatives -> backoff 120s ; dernier restart il y a 30s -> non.
        let last = now - chrono::Duration::seconds(30);
        assert!(!should_auto_restart(
            true,
            2,
            5,
            Some(last),
            now,
            DEFAULT_RESTART_BACKOFF_BASE_SECS,
            DEFAULT_RESTART_BACKOFF_CAP_SECS,
        ));
    }

    #[test]
    fn should_not_restart_when_disabled() {
        let now = Utc::now();
        // auto_restart_on_crash = false -> jamais de redemarrage, meme si
        // toutes les autres conditions sont favorables.
        assert!(!should_auto_restart(
            false,
            0,
            DEFAULT_MAX_RESTART_ATTEMPTS,
            None,
            now,
            DEFAULT_RESTART_BACKOFF_BASE_SECS,
            DEFAULT_RESTART_BACKOFF_CAP_SECS,
        ));
    }

    #[test]
    fn should_not_restart_when_cap_reached() {
        let now = Utc::now();
        let long_ago = now - chrono::Duration::days(1);
        assert!(!should_auto_restart(
            true,
            DEFAULT_MAX_RESTART_ATTEMPTS,
            DEFAULT_MAX_RESTART_ATTEMPTS,
            Some(long_ago),
            now,
            DEFAULT_RESTART_BACKOFF_BASE_SECS,
            DEFAULT_RESTART_BACKOFF_CAP_SECS,
        ));
        assert!(!should_auto_restart(
            true,
            DEFAULT_MAX_RESTART_ATTEMPTS + 3,
            DEFAULT_MAX_RESTART_ATTEMPTS,
            None,
            now,
            DEFAULT_RESTART_BACKOFF_BASE_SECS,
            DEFAULT_RESTART_BACKOFF_CAP_SECS,
        ));
    }

    #[test]
    fn should_restart_exactly_at_cooldown_boundary() {
        let now = Utc::now();
        // attempts=1 -> backoff 60s ; pile a la frontiere -> autorise (>=).
        let last = now - chrono::Duration::seconds(60);
        assert!(should_auto_restart(
            true,
            1,
            5,
            Some(last),
            now,
            DEFAULT_RESTART_BACKOFF_BASE_SECS,
            DEFAULT_RESTART_BACKOFF_CAP_SECS,
        ));
    }

    #[test]
    fn status_str_roundtrip() {
        for st in [
            GameServerStatus::Created,
            GameServerStatus::Scheduled,
            GameServerStatus::Starting,
            GameServerStatus::Running,
            GameServerStatus::Stopping,
            GameServerStatus::Stopped,
            GameServerStatus::Error,
            GameServerStatus::Deleted,
        ] {
            assert_eq!(GameServerStatus::from_str(st.as_str()), Some(st));
        }
    }

    // ── Auto-start des serveurs programmes ──

    #[test]
    fn auto_start_se_declenche_dans_la_fenetre_de_prep() {
        let now = Utc::now();
        // Revelation dans 4 min : on est deja dans la fenetre des 5 min -> start.
        let reveal = now + chrono::Duration::minutes(4);
        assert!(should_auto_start_scheduled(Some(reveal), now));
        // Revelation dans 6 min : trop tot, on attend encore.
        let reveal = now + chrono::Duration::minutes(6);
        assert!(!should_auto_start_scheduled(Some(reveal), now));
    }

    #[test]
    fn auto_start_a_la_frontiere_exacte_des_5_min() {
        let now = Utc::now();
        let reveal = now + chrono::Duration::minutes(PREP_LEAD_MINUTES);
        assert!(should_auto_start_scheduled(Some(reveal), now));
    }

    #[test]
    fn sans_heure_programmee_jamais_d_auto_start() {
        assert!(!should_auto_start_scheduled(None, Utc::now()));
    }

    // ── Marge memoire du conteneur ──

    /// Le bug qui empechait tout serveur Minecraft de demarrer : tas et limite
    /// du conteneur identiques, donc mort par le noyau des le lancement.
    #[test]
    fn le_conteneur_recoit_toujours_plus_que_le_jeu() {
        for tas in [512, 1024, 2048, 4096, 8192] {
            assert!(
                container_memory_mb(tas) > tas,
                "un tas de {tas} Mo doit obtenir un conteneur plus grand"
            );
        }
    }

    /// Les couts fixes d'une JVM ne diminuent pas avec le tas : un quart de
    /// 512 Mo ne suffirait pas.
    #[test]
    fn un_petit_serveur_obtient_le_plancher() {
        assert_eq!(container_memory_mb(512), 512 + 512);
        assert_eq!(container_memory_mb(1024), 1024 + 512);
    }

    /// Au-dela, la marge suit le tas : 2 Go de tas demandent plus de structures
    /// internes que 512 Mo.
    #[test]
    fn un_gros_serveur_obtient_un_quart_de_marge() {
        assert_eq!(container_memory_mb(4096), 4096 + 1024);
        assert_eq!(container_memory_mb(8192), 8192 + 2048);
    }

    /// La bascule entre plancher et proportion se fait a 2048 Mo, ou les deux
    /// valent 512.
    #[test]
    fn la_bascule_est_continue() {
        assert_eq!(container_memory_mb(2048), 2048 + 512);
        assert!(container_memory_mb(2049) >= container_memory_mb(2048));
    }

    /// Une valeur absurde ne doit pas deborder l'entier.
    #[test]
    fn une_valeur_extreme_ne_deborde_pas() {
        assert!(container_memory_mb(i32::MAX) >= i32::MAX - 1);
    }
}
