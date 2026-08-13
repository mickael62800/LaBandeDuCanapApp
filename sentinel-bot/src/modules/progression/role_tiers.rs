//! Attribution des roles de palier — ADAPTATEUR Discord.
//!
//! La decision (quels roles ajouter, lesquels retirer) vit dans le core
//! (`platform_core::sentinel::domain::services::progression::role_tiers`) avec ses tests.
//! Ce module ne fait que l'orchestration : lire la config, lire les roles
//! actuels du membre, et n'appeler Discord que pour les differences.

use serenity::all::{Context, GuildId, RoleId, UserId};
use tracing::{info, warn};

use platform_core::sentinel::domain::services::progression::role_tiers::{
    analyser_paliers, roles_pour_niveau, ModePalier,
};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::MODULE_BOT_NAME;

/// Applique les paliers de roles pour un membre a son niveau courant.
///
/// Best-effort de bout en bout : un echec ne remonte pas et n'interrompt
/// jamais le gain d'XP. Perdre un role est genant, perdre la progression du
/// membre le serait davantage.
pub async fn appliquer_paliers(ctx: &Context, guild_id: GuildId, user_id: UserId, niveau: i32) {
    let config = {
        let data = ctx.data.read().await;
        let Some(base) = data.get::<ApiClientKey>() else {
            return;
        };
        match base
            .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, %guild_id, "paliers: config illisible");
                return;
            }
        }
    };

    let paliers = analyser_paliers(&BaseApiClient::config_or(&config, "level_role_rewards", ""));
    if paliers.is_empty() {
        return;
    }
    let mode = ModePalier::depuis_config(&BaseApiClient::config_or(
        &config,
        "level_role_mode",
        "cumulatif",
    ));

    let (a_ajouter, a_retirer) = roles_pour_niveau(&paliers, niveau, mode);

    // Les roles actuels du membre : sans eux, on redemanderait a Discord
    // d'ajouter un role deja porte a chaque message, et le journal d'audit du
    // serveur se remplirait de mouvements sans objet.
    let membre = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, %guild_id, %user_id, "paliers: membre introuvable");
            return;
        }
    };
    let porte: Vec<u64> = membre.roles.iter().map(|r| r.get()).collect();

    for role in a_ajouter.into_iter().filter(|r| !porte.contains(r)) {
        match membre.add_role(&ctx.http, RoleId::new(role)).await {
            Ok(()) => info!(%guild_id, %user_id, role, niveau, "palier: role attribue"),
            Err(e) => warn!(error = %e, %guild_id, %user_id, role, "palier: echec attribution"),
        }
    }

    for role in a_retirer.into_iter().filter(|r| porte.contains(r)) {
        match membre.remove_role(&ctx.http, RoleId::new(role)).await {
            Ok(()) => info!(%guild_id, %user_id, role, niveau, "palier: role retire"),
            Err(e) => warn!(error = %e, %guild_id, %user_id, role, "palier: echec retrait"),
        }
    }
}

// ── Verification periodique ──

/// Throttle entre deux membres, aligne sur `/progression-resync`.
const INTERVALLE_MEMBRE_MS: u64 = 250;

/// Plafond de membres verifies par passage, du plus haut XP au plus bas.
const MEMBRES_PAR_PASSAGE: u32 = 200;

/// Verifie periodiquement que les roles correspondent aux niveaux.
///
/// Le level-up applique deja les paliers au moment ou il survient. Cette
/// boucle rattrape ce qu'un evenement ne peut pas voir : un role retire a la
/// main, un ajout de palier dans la configuration, une panne du bot pendant un
/// level-up, un role supprime puis recree.
///
/// # Rythme
///
/// Une heure par defaut, pas deux minutes. Chaque passage lit les roles de
/// chaque membre via l'API Discord ; a 200 membres, un passage toutes les deux
/// minutes represente 144 000 appels par jour pour ne rien changer la plupart
/// du temps, et Discord limite ces requetes. Le level-up reste immediat — la
/// boucle n'est qu'un filet, sa frequence n'a pas a etre celle du jeu.
///
/// Reglable par `ROLE_TIERS_RECONCILE_SECS`. Zero desactive la verification.
pub fn spawn_verification_periodique(ctx: Context) {
    use tokio::time::{interval, sleep, Duration};

    let periode = std::env::var("ROLE_TIERS_RECONCILE_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(3600);
    if periode == 0 {
        info!("paliers: verification periodique desactivee");
        return;
    }

    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(periode.max(60)));
        tick.tick().await; // le premier tick est immediat, on l'ignore
        loop {
            tick.tick().await;

            for guild_id in ctx.cache.guilds() {
                let membres = match classement(&ctx, guild_id).await {
                    Some(m) => m,
                    None => continue,
                };
                for (user_id, niveau) in membres {
                    appliquer_paliers(&ctx, guild_id, user_id, niveau).await;
                    sleep(Duration::from_millis(INTERVALLE_MEMBRE_MS)).await;
                }
            }
        }
    });
}

/// Les membres a verifier, du plus haut XP au plus bas.
///
/// `None` quand rien n'est a faire : API indisponible, ou aucun palier
/// configure — inutile de parcourir la guilde pour n'appliquer aucune regle.
async fn classement(ctx: &Context, guild_id: GuildId) -> Option<Vec<(UserId, i32)>> {
    // Le verrou ne couvre que ces deux lectures. La boucle qui suit, elle,
    // dure des dizaines de secondes et s'execute en dehors : la tenir sous
    // verrou bloquerait les ecritures sur `ctx.data` pour tout le bot.
    let data = ctx.data.read().await;
    let base = data.get::<ApiClientKey>()?;
    let api = data.get::<super::StatsApiKey>()?;

    let config = base
        .get_guild_config_for(&guild_id.to_string(), MODULE_BOT_NAME)
        .await
        .ok()?;
    if analyser_paliers(&BaseApiClient::config_or(&config, "level_role_rewards", "")).is_empty() {
        return None;
    }

    let liste = api
        .get_level_leaderboard(&guild_id.to_string(), MEMBRES_PAR_PASSAGE, None)
        .await
        .ok()?;

    Some(
        liste
            .into_iter()
            .filter_map(|e| {
                e.user_id
                    .parse::<u64>()
                    .ok()
                    .map(|id| (UserId::new(id), e.level))
            })
            .collect(),
    )
}
