//! Salons compteurs de Nexus : « En jeu : 7 », « Serveurs actifs : 2 ».
//!
//! Meme principe que les compteurs de l'accueil : un salon dont le NOM porte
//! le chiffre. Discord n'accepte que **deux renommages par salon et par
//! tranche de dix minutes** ; au-dela, la requete est mise en file et le nom
//! reste faux pendant longtemps. D'ou deux precautions : un rafraichissement
//! espace, et surtout aucune ecriture quand le nom ne change pas.
//!
//! POURQUOI DEUX COMPTEURS. Le nombre de joueurs vient de la console du jeu,
//! que la plateforme ne sait lire que pour Minecraft et Palworld. Ailleurs le
//! comptage vaut zero — pas parce que le serveur est vide, mais parce que
//! personne ne sait le demander. Le compteur de serveurs, lui, ne depend
//! d'aucune console et reste vrai pour tout le catalogue.

use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{ActivityType, ChannelId, Context, EditChannel, GuildId};

use crate::api_client::{ApiClient, GameServer};

/// Periode de rafraichissement. Cinq minutes tient dans le quota Discord
/// (deux renommages par dix minutes) tout en laissant deux ecritures possibles
/// si les deux compteurs bougent.
const PERIODE: std::time::Duration = std::time::Duration::from_secs(300);

const MODULE_BOT_NAME: &str = "game-portal";

/// L'intent de presence est-il demande par l'exploitant ?
///
/// Interrupteur volontairement separe du reglage de guilde : celui-ci dit
/// « affiche ce compteur », celle-la dit « le bot a le droit de lire les
/// activites ». Confondre les deux ferait qu'activer un compteur dans le
/// tableau de bord tenterait de changer les intents d'un bot deja connecte —
/// ce qui n'arrive qu'au demarrage, et peut lui couter sa connexion.
pub fn intent_presence_demande() -> bool {
    matches!(
        std::env::var("NEXUS_PRESENCE_INTENT")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "oui"
    )
}

/// Un membre est-il en train de jouer ?
///
/// Discord range sous « activite » aussi bien un jeu qu'un statut personnalise,
/// une ecoute Spotify ou un flux en direct. Seul `Playing` designe une partie
/// en cours ; compter les autres gonflerait le chiffre avec des gens qui
/// ecoutent de la musique.
///
/// Un membre qui joue a deux choses a la fois ne compte qu'une fois : on
/// compte des personnes, pas des parties.
pub fn est_en_jeu(activites: &[ActivityType]) -> bool {
    activites.contains(&ActivityType::Playing)
}

/// Ce qu'il y a a afficher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Releve {
    pub joueurs: i64,
    pub serveurs: i64,
}

/// Compte les membres qui jouent ET sont en vocal sur ce serveur.
///
/// Les autres compteurs disent combien de personnes jouent, ou combien de
/// machines tournent. Celui-ci dit combien jouent ENSEMBLE : deux personnes qui
/// jouent chacune dans leur coin comptent pour deux dans « en partie », et pour
/// zero ici.
///
/// Meme reserve que `membres_en_activite` : `None` quand l'information n'est
/// pas disponible, ce qui n'est pas zero.
pub fn membres_en_jeu_et_en_vocal(ctx: &Context, guild_id: GuildId) -> Option<i64> {
    if !intent_presence_demande() {
        return None;
    }
    let guilde = ctx.cache.guild(guild_id)?;
    let total = guilde
        .voice_states
        .iter()
        // Un etat vocal sans salon, c'est quelqu'un qui vient de raccrocher :
        // Discord garde l'entree un instant.
        .filter(|(_, etat)| etat.channel_id.is_some())
        .filter(|(user_id, _)| {
            let humain = guilde.members.get(*user_id).map(|m| !m.user.bot);
            humain.unwrap_or(false)
                && guilde
                    .presences
                    .get(*user_id)
                    .map(|presence| {
                        est_en_jeu(
                            &presence
                                .activities
                                .iter()
                                .map(|a| a.kind)
                                .collect::<Vec<_>>(),
                        )
                    })
                    .unwrap_or(false)
        })
        .count();
    Some(total as i64)
}

/// Compte les membres de la guilde qui jouent a QUELQUE CHOSE, serveurs Nexus
/// compris ou non — c'est l'activite Discord qui fait foi.
///
/// Renvoie `None` quand l'information n'est pas disponible : intent de
/// presence eteint, ou guilde absente du cache. Ne pas confondre avec zero —
/// afficher « 0 en jeu » parce qu'on n'a pas le droit de regarder serait
/// mensonger, et le compteur resterait fige a zero sans que personne ne
/// comprenne pourquoi.
///
/// Les bots sont exclus : un bot musique « joue » en permanence.
pub fn membres_en_activite(ctx: &Context, guild_id: GuildId) -> Option<i64> {
    if !intent_presence_demande() {
        return None;
    }
    let guilde = ctx.cache.guild(guild_id)?;
    let total = guilde
        .presences
        .iter()
        .filter(|(user_id, presence)| {
            // La presence ne dit pas si le compte est un bot : on le demande
            // au cache des membres, et on ignore l'entree si elle est inconnue
            // plutot que de risquer de compter un bot musique.
            let humain = guilde.members.get(*user_id).map(|m| !m.user.bot);
            humain.unwrap_or(false)
                && est_en_jeu(
                    &presence
                        .activities
                        .iter()
                        .map(|a| a.kind)
                        .collect::<Vec<_>>(),
                )
        })
        .count();
    Some(total as i64)
}

/// Compte ce qui tourne vraiment.
///
/// Seuls les serveurs EN LIGNE comptent : un serveur programme pour ce soir,
/// en cours de demarrage ou arrete n'accueille personne, et le faire figurer
/// annoncerait une soiree qui n'a pas commence.
///
/// Un comptage negatif ne devrait jamais arriver ; s'il arrivait, il ferait un
/// total plus petit que la realite. On le ramene a zero.
pub fn relever(serveurs: &[GameServer]) -> Releve {
    let en_ligne = serveurs.iter().filter(|s| s.status == "running");
    Releve {
        joueurs: en_ligne
            .clone()
            .map(|s| i64::from(s.last_player_count.max(0)))
            .sum(),
        serveurs: en_ligne.count() as i64,
    }
}

/// Applique le chiffre au format choisi par le serveur.
///
/// Un format sans `{count}` donnerait un nom fixe : le compteur cesserait
/// d'etre un compteur, sans que rien ne le signale. On le refuse plutot que
/// d'ecrire un nom mort dans Discord.
pub fn nom_du_salon(format: &str, valeur: i64) -> Option<String> {
    if !format.contains("{count}") {
        return None;
    }
    let nom = format.replace("{count}", &valeur.to_string());
    // Discord tronque au-dela de 100 caracteres ; on tronque nous-memes pour
    // que la comparaison « le nom a-t-il change ? » porte sur ce qui sera
    // reellement ecrit, sinon chaque passage croirait devoir reecrire.
    Some(nom.chars().take(100).collect())
}

/// Renomme le salon, seulement si son nom doit changer.
async fn appliquer(ctx: &Context, salon: ChannelId, nom: &str) {
    let actuel = salon
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild().map(|g| g.name));
    // Le quota de renommage est la vraie ressource rare ici : une ecriture
    // inutile toutes les cinq minutes le consommerait entierement, et le jour
    // ou le chiffre change, Discord ferait attendre la mise a jour.
    if actuel.as_deref() == Some(nom) {
        return;
    }
    if let Err(error) = salon.edit(&ctx.http, EditChannel::new().name(nom)).await {
        tracing::warn!(%error, %salon, "compteurs: renommage impossible");
    }
}

fn salon_configure(cfg: &HashMap<String, String>, cle: &str) -> Option<ChannelId> {
    cfg.get(cle)
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .and_then(|v| v.parse::<u64>().ok())
        .map(ChannelId::new)
}

fn format_configure<'a>(cfg: &'a HashMap<String, String>, cle: &str, defaut: &'a str) -> String {
    cfg.get(cle)
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .unwrap_or(defaut)
        .to_string()
}

/// Met a jour les deux compteurs d'une guilde.
pub async fn rafraichir(ctx: &Context, api: &ApiClient, guild_id: GuildId) {
    let guild_key = guild_id.to_string();
    let cfg = match api.get_guild_config(&guild_key, MODULE_BOT_NAME).await {
        Ok(c) => c,
        Err(error) => {
            tracing::warn!(%error, guild = %guild_id, "compteurs: configuration illisible");
            return;
        }
    };

    let salon_joueurs = salon_configure(&cfg, "players_counter_channel_id");
    let salon_serveurs = salon_configure(&cfg, "servers_counter_channel_id");
    let salon_activite = salon_configure(&cfg, "activity_counter_channel_id");
    let salon_ensemble = salon_configure(&cfg, "ingame_voice_counter_channel_id");
    if salon_joueurs.is_none()
        && salon_serveurs.is_none()
        && salon_activite.is_none()
        && salon_ensemble.is_none()
    {
        return;
    }

    if let Some(salon) = salon_ensemble {
        match membres_en_jeu_et_en_vocal(ctx, guild_id) {
            Some(total) => {
                if let Some(nom) = nom_du_salon(
                    &format_configure(&cfg, "ingame_voice_counter_format", "🎧 Ensemble : {count}"),
                    total,
                ) {
                    appliquer(ctx, salon, &nom).await;
                }
            }
            None => tracing::debug!(
                guild = %guild_id,
                "compteurs: presence indisponible, compteur « en jeu et en vocal » laisse tel quel"
            ),
        }
    }

    // Compteur « tous jeux confondus » : il ne depend pas de l'API, seulement
    // de ce que Discord veut bien dire. Traite a part, et avant, pour qu'une
    // API muette ne l'empeche pas de vivre.
    if let Some(salon) = salon_activite {
        match membres_en_activite(ctx, guild_id) {
            Some(total) => {
                if let Some(nom) = nom_du_salon(
                    &format_configure(&cfg, "activity_counter_format", "🕹️ En partie : {count}"),
                    total,
                ) {
                    appliquer(ctx, salon, &nom).await;
                }
            }
            // Pas le droit de regarder, ou guilde pas encore en cache : on ne
            // touche pas au salon. Ecrire zero ferait croire que personne ne
            // joue, alors qu'on ne sait simplement pas.
            None => tracing::debug!(
                guild = %guild_id,
                "compteurs: activite des membres indisponible (intent de presence eteint ?)"
            ),
        }
    }

    let serveurs = match api.list_game_servers(&guild_key).await {
        Ok(s) => s,
        Err(error) => {
            // On ne touche a rien : garder le dernier chiffre affiche vaut
            // mieux que d'ecrire un zero qui ferait croire le serveur desert.
            tracing::warn!(%error, guild = %guild_id, "compteurs: serveurs illisibles");
            return;
        }
    };
    let releve = relever(&serveurs);

    if let Some(salon) = salon_joueurs {
        if let Some(nom) = nom_du_salon(
            &format_configure(&cfg, "players_counter_format", "🎮 En jeu : {count}"),
            releve.joueurs,
        ) {
            appliquer(ctx, salon, &nom).await;
        } else {
            tracing::warn!(
                guild = %guild_id,
                "compteurs: le format des joueurs ne contient pas le marqueur de nombre, salon laisse tel quel"
            );
        }
    }

    if let Some(salon) = salon_serveurs {
        if let Some(nom) = nom_du_salon(
            &format_configure(
                &cfg,
                "servers_counter_format",
                "🖥️ Serveurs actifs : {count}",
            ),
            releve.serveurs,
        ) {
            appliquer(ctx, salon, &nom).await;
        } else {
            tracing::warn!(
                guild = %guild_id,
                "compteurs: le format des serveurs ne contient pas le marqueur de nombre, salon laisse tel quel"
            );
        }
    }
}

/// Lance le rafraichissement periodique. Appele une fois au `ready`.
pub fn spawn(ctx: Context, api: Arc<ApiClient>, guild_ids: Vec<GuildId>) {
    tokio::spawn(async move {
        // Le compteur d'activite lit le cache des membres et des presences,
        // que Discord remplit en plusieurs envois apres la connexion. Compter
        // immediatement donnerait un chiffre trop bas, ecrit dans le nom d'un
        // salon — et il y resterait jusqu'au passage suivant.
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        let mut horloge = tokio::time::interval(PERIODE);
        loop {
            horloge.tick().await;
            for guild_id in &guild_ids {
                rafraichir(&ctx, &api, *guild_id).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serveur(status: &str, joueurs: i32) -> GameServer {
        GameServer {
            id: "s".into(),
            guild_id: "g".into(),
            template_id: "t".into(),
            name: "n".into(),
            status: status.into(),
            owner_user_id: "o".into(),
            host_port: None,
            public_host: None,
            ip_reveal_at: None,
            ip_revealed: false,
            display_state: None,
            text_channel_id: None,
            voice_channel_id: None,
            last_player_count: joueurs,
        }
    }

    #[test]
    fn seuls_les_serveurs_en_ligne_sont_comptes() {
        // Un serveur programme pour ce soir n'accueille personne : l'afficher
        // annoncerait une soiree qui n'a pas commence.
        let releve = relever(&[
            serveur("running", 4),
            serveur("running", 3),
            serveur("scheduled", 0),
            serveur("stopped", 0),
            serveur("starting", 0),
        ]);
        assert_eq!(releve.joueurs, 7);
        assert_eq!(releve.serveurs, 2);
    }

    #[test]
    fn un_serveur_sans_console_lisible_compte_zero_joueur_mais_un_serveur() {
        // Valheim ne repond pas a la console : son comptage vaut zero. C'est
        // exactement la raison d'etre du second compteur — sans lui, une
        // soiree pleine s'afficherait « 0 en jeu » et rien d'autre.
        let releve = relever(&[serveur("running", 0)]);
        assert_eq!(releve.joueurs, 0);
        assert_eq!(releve.serveurs, 1);
    }

    #[test]
    fn le_total_ignore_un_comptage_negatif() {
        let releve = relever(&[serveur("running", -5), serveur("running", 2)]);
        assert_eq!(releve.joueurs, 2);
    }

    #[test]
    fn seule_une_partie_en_cours_compte_comme_du_jeu() {
        // Discord range sous « activite » le statut personnalise, Spotify et
        // les flux en direct. Les compter gonflerait le chiffre avec des gens
        // qui ecoutent de la musique.
        assert!(est_en_jeu(&[ActivityType::Playing]));
        assert!(est_en_jeu(&[ActivityType::Custom, ActivityType::Playing]));
        assert!(!est_en_jeu(&[ActivityType::Listening]));
        assert!(!est_en_jeu(&[ActivityType::Custom, ActivityType::Watching]));
        assert!(!est_en_jeu(&[ActivityType::Streaming]));
        assert!(!est_en_jeu(&[]));
    }

    #[test]
    fn un_membre_qui_joue_a_deux_choses_ne_compte_qu_une_fois() {
        // On compte des personnes, pas des parties.
        assert!(est_en_jeu(&[ActivityType::Playing, ActivityType::Playing]));
    }

    #[test]
    fn le_format_place_le_chiffre() {
        assert_eq!(
            nom_du_salon("🎮 En jeu : {count}", 7).as_deref(),
            Some("🎮 En jeu : 7")
        );
    }

    #[test]
    fn un_format_sans_marqueur_est_refuse() {
        // Sinon le bot ecrirait un nom fixe et consommerait le quota de
        // renommage pour rien, en donnant l'illusion d'un compteur.
        assert_eq!(nom_du_salon("En jeu", 7), None);
    }

    #[test]
    fn un_nom_trop_long_est_tronque_comme_le_fera_discord() {
        // Sans cette coupe, la comparaison porterait sur un nom que Discord
        // n'accepterait pas tel quel : chaque passage se croirait oblige de
        // reecrire, et le quota fondrait.
        let format = format!("{}{{count}}", "a".repeat(120));
        let nom = nom_du_salon(&format, 3).unwrap();
        assert_eq!(nom.chars().count(), 100);
    }

    /// L'intent de presence se lit dans une variable d'environnement, donc
    /// GLOBALE au processus. Les tests tournant en parallele, un seul test
    /// couvre toute la variable : deux tests qui la posent puis la retirent se
    /// marcheraient dessus, et l'un des deux echouerait au hasard.
    #[test]
    fn l_intent_de_presence_se_lit_de_facon_permissive() {
        // Absente : fail closed, le bot ne reclame pas l'intent.
        std::env::remove_var("NEXUS_PRESENCE_INTENT");
        assert!(!intent_presence_demande());

        for valeur in ["1", "true", "TRUE", "yes", "YES", "oui", "OUI"] {
            std::env::set_var("NEXUS_PRESENCE_INTENT", valeur);
            assert!(intent_presence_demande(), "« {valeur} » devrait activer");
        }

        // Les espaces autour ne changent rien : un .env se saisit a la main.
        for valeur in ["  true  ", "  oui  "] {
            std::env::set_var("NEXUS_PRESENCE_INTENT", valeur);
            assert!(intent_presence_demande(), "« {valeur} » devrait activer");
        }

        for valeur in ["0", "false", "no", "random", ""] {
            std::env::set_var("NEXUS_PRESENCE_INTENT", valeur);
            assert!(
                !intent_presence_demande(),
                "« {valeur} » ne doit pas activer"
            );
        }

        std::env::remove_var("NEXUS_PRESENCE_INTENT");
    }

    #[test]
    fn salon_configure_found() {
        let mut cfg = HashMap::new();
        cfg.insert("joueurs".into(), "123456789".into());
        assert_eq!(
            salon_configure(&cfg, "joueurs"),
            Some(ChannelId::new(123456789))
        );
    }

    #[test]
    fn salon_configure_not_found() {
        let cfg = HashMap::new();
        assert_eq!(salon_configure(&cfg, "joueurs"), None);
    }

    #[test]
    fn salon_configure_empty_value() {
        let mut cfg = HashMap::new();
        cfg.insert("joueurs".into(), "".into());
        assert_eq!(salon_configure(&cfg, "joueurs"), None);
    }

    #[test]
    fn salon_configure_whitespace_only() {
        let mut cfg = HashMap::new();
        cfg.insert("joueurs".into(), "   ".into());
        assert_eq!(salon_configure(&cfg, "joueurs"), None);
    }

    #[test]
    fn salon_configure_with_padding() {
        let mut cfg = HashMap::new();
        cfg.insert("joueurs".into(), "  999  ".into());
        assert_eq!(salon_configure(&cfg, "joueurs"), Some(ChannelId::new(999)));
    }

    #[test]
    fn nom_du_salon_zero_joueurs() {
        assert_eq!(nom_du_salon("🎮 {count}", 0).as_deref(), Some("🎮 0"));
    }

    #[test]
    fn nom_du_salon_large_number() {
        assert_eq!(
            nom_du_salon("Joueurs: {count}", 123456).as_deref(),
            Some("Joueurs: 123456")
        );
    }

    #[test]
    fn nom_du_salon_unicode() {
        assert_eq!(
            nom_du_salon("🎮 En jeu: {count} 👾", 5).as_deref(),
            Some("🎮 En jeu: 5 👾")
        );
    }

    #[test]
    fn nom_du_salon_multiple_placeholders() {
        // Seule la première occurence devrait être remplacée (behavior du replace)
        assert_eq!(
            nom_du_salon("{count} / {count}", 3).as_deref(),
            Some("3 / 3")
        );
    }

    #[test]
    fn relever_no_servers() {
        let releve = relever(&[]);
        assert_eq!(releve.joueurs, 0);
        assert_eq!(releve.serveurs, 0);
    }

    #[test]
    fn relever_all_offline() {
        let releve = relever(&[
            serveur("stopped", 0),
            serveur("scheduled", 0),
            serveur("starting", 0),
        ]);
        assert_eq!(releve.joueurs, 0);
        assert_eq!(releve.serveurs, 0);
    }

    #[test]
    fn relever_mixed_running_and_offline() {
        let releve = relever(&[
            serveur("running", 10),
            serveur("running", 20),
            serveur("stopped", 100), // Shouldn't count
            serveur("starting", 50), // Shouldn't count
        ]);
        assert_eq!(releve.joueurs, 30);
        assert_eq!(releve.serveurs, 2);
    }

    #[test]
    fn relever_clamping_zero_for_negative() {
        let releve = relever(&[serveur("running", -10), serveur("running", -5)]);
        assert_eq!(releve.joueurs, 0);
        assert_eq!(releve.serveurs, 2);
    }

    #[test]
    fn est_en_jeu_empty_vec() {
        assert!(!est_en_jeu(&[]));
    }

    #[test]
    fn est_en_jeu_only_playing() {
        assert!(est_en_jeu(&[ActivityType::Playing]));
    }

    #[test]
    fn est_en_jeu_multiple_playing() {
        assert!(est_en_jeu(&[
            ActivityType::Playing,
            ActivityType::Playing,
            ActivityType::Playing
        ]));
    }

    #[test]
    fn est_en_jeu_mixed_with_other_activities() {
        assert!(est_en_jeu(&[
            ActivityType::Listening,
            ActivityType::Playing,
            ActivityType::Watching
        ]));
    }

    #[test]
    fn est_en_jeu_no_playing() {
        assert!(!est_en_jeu(&[
            ActivityType::Listening,
            ActivityType::Watching,
            ActivityType::Streaming
        ]));
    }

    #[test]
    fn releve_default_struct() {
        let r = Releve::default();
        assert_eq!(r.joueurs, 0);
        assert_eq!(r.serveurs, 0);
    }

    #[test]
    fn releve_equality() {
        let r1 = Releve {
            joueurs: 5,
            serveurs: 2,
        };
        let r2 = Releve {
            joueurs: 5,
            serveurs: 2,
        };
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_releve_zero_values() {
        let r = Releve {
            joueurs: 0,
            serveurs: 0,
        };
        assert_eq!(r.joueurs, 0);
        assert_eq!(r.serveurs, 0);
    }

    #[test]
    fn test_releve_large_values() {
        let r = Releve {
            joueurs: 9999,
            serveurs: 999,
        };
        assert_eq!(r.joueurs, 9999);
        assert_eq!(r.serveurs, 999);
    }

    #[test]
    fn test_releve_inequality() {
        let r1 = Releve {
            joueurs: 5,
            serveurs: 2,
        };
        let r2 = Releve {
            joueurs: 6,
            serveurs: 2,
        };
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_nom_du_salon_edge_cases() {
        // Minimum format
        assert_eq!(nom_du_salon("{count}", 0), Some("0".into()));
        assert_eq!(nom_du_salon("{count}", 1), Some("1".into()));

        // Very large number
        assert_eq!(nom_du_salon("Players: {count}", 99999), Some("Players: 99999".into()));
    }


    #[test]
    fn test_le_format_place_le_chiffre_variations() {
        assert_eq!(nom_du_salon("({count})", 42), Some("(42)".into()));
        assert_eq!(nom_du_salon("[{count}]", 99), Some("[99]".into()));
        assert_eq!(nom_du_salon("# {count}", 7), Some("# 7".into()));
    }
}
