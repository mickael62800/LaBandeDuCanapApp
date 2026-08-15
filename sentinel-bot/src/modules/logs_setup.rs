//! Installation en un clic des salons de logs.
//!
//! Douze journaux repartis sur cinq modules, chacun avec sa cle de reglage.
//! Les creer a la main puis coller douze identifiants dans le back-office est
//! long et se prete aux erreurs — un identifiant colle dans le mauvais champ
//! ne produit aucune erreur, juste un journal muet qu'on decouvre des semaines
//! plus tard.
//!
//! La commande fait donc les trois etapes d'un coup : creer la categorie,
//! creer les salons manquants, ecrire les reglages.
//!
//! # Idempotence
//!
//! Relancable sans risque. Un salon deja reference dans la configuration ET
//! toujours vivant sur Discord est conserve tel quel ; un salon portant le bon
//! nom dans la categorie est adopte plutot que double ; seul ce qui manque est
//! cree. On peut donc relancer apres avoir supprime un salon a la main pour
//! le voir recree, sans toucher aux autres.

use std::sync::Arc;

use serenity::all::{
    ChannelId, ChannelType, CommandInteraction, Context, CreateChannel, CreateCommand, CreateEmbed,
    GuildId, PermissionOverwrite, PermissionOverwriteType, Permissions, RoleId,
};
use tracing::warn;

use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::{has_manage_guild, reply_ephemeral};
use crate::shared::heartbeat::ApiClientKey;

/// Nom de la categorie creee si elle n'existe pas encore.
const NOM_CATEGORIE: &str = "📋 Logs";

/// Un journal : le salon a creer, et le reglage a renseigner avec son id.
struct Journal {
    /// Nom du salon Discord (minuscules et tirets, contrainte Discord).
    nom: &'static str,
    /// Composant proprietaire du reglage.
    module: &'static str,
    /// Cle de configuration recevant l'identifiant du salon.
    cle: &'static str,
    /// Ce que le salon recoit, affiche dans le compte rendu.
    quoi: &'static str,
}

/// Les douze journaux, dans l'ordre ou ils seront crees.
///
/// L'ordre compte : Discord place les salons dans leur ordre de creation, et
/// c'est celui-ci qu'on veut lire dans la categorie — moderation d'abord,
/// activite ensuite, technique a la fin.
const JOURNAUX: &[Journal] = &[
    Journal {
        nom: "log-sanctions",
        module: "moderation-bot",
        cle: "sanctions_log_channel_id",
        quoi: "recap des sanctions",
    },
    Journal {
        nom: "log-moderation",
        module: "moderation-bot",
        cle: "log_channel_id",
        quoi: "carte detaillee des sanctions",
    },
    Journal {
        nom: "log-automod",
        module: "automod-bot",
        cle: "log_channel_id",
        quoi: "detections et votes automod",
    },
    Journal {
        nom: "log-bans-age",
        module: "welcome-bot",
        cle: "age_ban_log_channel_id",
        quoi: "bans par verification d'age",
    },
    Journal {
        nom: "log-arrivees",
        module: "audit-bot",
        cle: "join_leave_channel_id",
        quoi: "arrivees et departs",
    },
    Journal {
        nom: "log-profils",
        module: "audit-bot",
        cle: "profile_edit_channel_id",
        quoi: "pseudos, avatars, roles",
    },
    Journal {
        nom: "log-messages",
        module: "audit-bot",
        cle: "message_log_channel_id",
        quoi: "editions et suppressions",
    },
    Journal {
        nom: "log-vocal-activite",
        module: "audit-bot",
        cle: "voice_log_channel_id",
        quoi: "connexions et deconnexions vocales",
    },
    Journal {
        nom: "log-vocaux",
        module: "voice-bot",
        cle: "log_channel_id",
        quoi: "sessions des salons temporaires",
    },
    Journal {
        nom: "log-commandes",
        module: "audit-bot",
        cle: "command_log_channel_id",
        quoi: "commandes admin executees",
    },
    Journal {
        nom: "log-annonces",
        module: "announcements",
        cle: "log_channel_id",
        quoi: "publications planifiees",
    },
    Journal {
        nom: "log-alertes-jeux",
        module: "ops-api",
        cle: "game_alerts_log_channel_id",
        quoi: "alertes RAM/CPU et statut des serveurs de jeu",
    },
    Journal {
        nom: "log-general",
        module: "audit-bot",
        cle: "log_channel_id",
        quoi: "repli : tout ce qui n'a pas de salon dedie",
    },
];

pub fn register() -> CreateCommand {
    CreateCommand::new("logs-init")
        .description("Cree la categorie et tous les salons de logs, puis les configure")
        .default_member_permissions(Permissions::MANAGE_GUILD)
}

/// Ce qui est arrive a un journal, pour le compte rendu.
enum Issue {
    Cree,
    Reutilise,
    Echec(String),
}

pub async fn handle(ctx: &Context, cmd: &CommandInteraction) {
    if !has_manage_guild(cmd) {
        reply_ephemeral(
            ctx,
            cmd,
            "La permission MANAGE_GUILD est requise pour initialiser les salons de logs.",
        )
        .await;
        tracing::warn!(
            user = %cmd.user.name,
            user_id = %cmd.user.id,
            "Tentative /logs-init sans permission"
        );
        return;
    }

    let Some(guild_id) = cmd.guild_id else {
        return;
    };

    // L'operation cree jusqu'a treize salons : bien au-dela des trois secondes
    // accordees pour repondre a une interaction.
    if cmd.defer_ephemeral(&ctx.http).await.is_err() {
        return;
    }

    let api = {
        let data = ctx.data.read().await;
        data.get::<ApiClientKey>().cloned()
    };
    let Some(api) = api else {
        repondre(ctx, cmd, "API indisponible, reessaie plus tard.").await;
        return;
    };

    let categorie = match trouver_ou_creer_categorie(ctx, guild_id).await {
        Ok(id) => id,
        Err(e) => {
            repondre(ctx, cmd, &format!("Categorie impossible a creer : {e}")).await;
            return;
        }
    };

    // Un seul appel : la liste des salons sert a tous les journaux.
    let existants = match guild_id.channels(&ctx.http).await {
        Ok(c) => c,
        Err(e) => {
            repondre(ctx, cmd, &format!("Lecture des salons impossible : {e}")).await;
            return;
        }
    };

    let surcharges = surcharges_staff(guild_id);
    let mut resultats: Vec<(&Journal, Issue)> = Vec::new();

    for journal in JOURNAUX {
        // Salon deja porteur du bon nom dans n'importe quelle categorie de logs ou sur le serveur : on l'adopte sans dupliquer.
        let deja = existants.values().find(|c| {
            c.kind == ChannelType::Text && c.name.to_lowercase() == journal.nom.to_lowercase()
        });

        let issue = match deja {
            Some(ch) => {
                ecrire_reglage(&api, guild_id, journal, ch.id).await;
                Issue::Reutilise
            }
            None => {
                let builder = CreateChannel::new(journal.nom)
                    .kind(ChannelType::Text)
                    .category(categorie)
                    .topic(journal.quoi)
                    .permissions(surcharges.clone());
                match guild_id.create_channel(&ctx.http, builder).await {
                    Ok(ch) => {
                        ecrire_reglage(&api, guild_id, journal, ch.id).await;
                        Issue::Cree
                    }
                    Err(e) => Issue::Echec(e.to_string()),
                }
            }
        };
        resultats.push((journal, issue));
    }

    repondre_rapport(ctx, cmd, categorie, &resultats).await;
}

/// Categorie des logs : reutilise la categorie existante "📋 Logs" ou "🎬 Les Coulisses", sinon cree "📋 Logs".
async fn trouver_ou_creer_categorie(ctx: &Context, guild_id: GuildId) -> Result<ChannelId, String> {
    let salons = guild_id
        .channels(&ctx.http)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(c) = salons
        .values()
        .find(|c| c.kind == ChannelType::Category && (c.name.contains("Logs") || c.name.contains("Coulisses") || c.name == NOM_CATEGORIE))
    {
        return Ok(c.id);
    }

    guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new(NOM_CATEGORIE)
                .kind(ChannelType::Category)
                .permissions(surcharges_staff(guild_id)),
        )
        .await
        .map(|c| c.id)
        .map_err(|e| e.to_string())
}

/// @everyone ne voit rien.
///
/// Un journal expose les suppressions de messages, les motifs de sanction et
/// les identifiants des membres. Le laisser public le rendrait pire
/// qu'inutile. On ne donne d'acces a aucun role ici : le staff passe par ses
/// permissions d'administration, et l'operateur ajoutera ses roles de
/// moderation s'il veut les ouvrir plus largement.
fn surcharges_staff(guild_id: GuildId) -> Vec<PermissionOverwrite> {
    vec![PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL,
        // @everyone porte le meme identifiant que la guilde.
        kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
    }]
}

/// Ecrit l'identifiant du salon dans le reglage du module proprietaire.
async fn ecrire_reglage(
    api: &Arc<BaseApiClient>,
    guild_id: GuildId,
    journal: &Journal,
    salon: ChannelId,
) {
    let corps = serde_json::json!({
        "guild_id": guild_id.to_string(),
        "bot_name": journal.module,
        "config_key": journal.cle,
        "config_value": salon.to_string(),
    });
    api.post_fire_and_forget("/api/bots/config", &corps).await;
}

async fn repondre_rapport(
    ctx: &Context,
    cmd: &CommandInteraction,
    categorie: ChannelId,
    resultats: &[(&Journal, Issue)],
) {
    let crees = resultats
        .iter()
        .filter(|(_, i)| matches!(i, Issue::Cree))
        .count();
    let reutilises = resultats
        .iter()
        .filter(|(_, i)| matches!(i, Issue::Reutilise))
        .count();

    let mut lignes = String::new();
    for (journal, issue) in resultats {
        let marque = match issue {
            Issue::Cree => "🆕",
            Issue::Reutilise => "♻️",
            Issue::Echec(_) => "❌",
        };
        lignes.push_str(&format!("{marque} `#{}` — {}\n", journal.nom, journal.quoi));
    }

    let echecs: Vec<String> = resultats
        .iter()
        .filter_map(|(j, i)| match i {
            Issue::Echec(e) => Some(format!("`#{}` : {e}", j.nom)),
            _ => None,
        })
        .collect();

    let mut embed = CreateEmbed::new()
        .title("📋 Salons de logs")
        .description(format!(
            "Categorie <#{categorie}>\n\
             **{crees}** cree(s), **{reutilises}** deja en place.\n\n{lignes}"
        ))
        .colour(if echecs.is_empty() {
            0x2ECC71
        } else {
            0xE67E22
        });

    if !echecs.is_empty() {
        embed = embed.field("Echecs", echecs.join("\n"), false);
    }

    embed = embed.footer(serenity::all::CreateEmbedFooter::new(
        "Les salons sont masques a @everyone. Relancable sans risque.",
    ));

    if let Err(e) = cmd
        .create_followup(
            &ctx.http,
            serenity::all::CreateInteractionResponseFollowup::new()
                .embed(embed)
                .ephemeral(true),
        )
        .await
    {
        warn!(error = %e, "logs-init: echec envoi du rapport");
    }
}

async fn repondre(ctx: &Context, cmd: &CommandInteraction, message: &str) {
    let _ = cmd
        .create_followup(
            &ctx.http,
            serenity::all::CreateInteractionResponseFollowup::new()
                .content(message)
                .ephemeral(true),
        )
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_noms_de_salon_sont_valides_pour_discord() {
        // Discord impose minuscules, chiffres et tirets. Un nom invalide fait
        // echouer la creation avec un message peu parlant.
        for j in JOURNAUX {
            assert!(
                j.nom
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "nom invalide : {}",
                j.nom
            );
        }
    }

    #[test]
    fn aucun_nom_de_salon_en_double() {
        // Deux journaux de meme nom se voleraient mutuellement leur salon a
        // chaque relance, en alternance.
        let mut noms: Vec<&str> = JOURNAUX.iter().map(|j| j.nom).collect();
        noms.sort_unstable();
        let avant = noms.len();
        noms.dedup();
        assert_eq!(noms.len(), avant, "noms de salon dupliques");
    }

    #[test]
    fn aucune_cle_de_reglage_en_double_dans_un_module() {
        // Deux journaux ecrivant la meme cle du meme module : le second
        // ecraserait le premier, un salon resterait muet sans explication.
        let mut paires: Vec<(&str, &str)> = JOURNAUX.iter().map(|j| (j.module, j.cle)).collect();
        paires.sort_unstable();
        let avant = paires.len();
        paires.dedup();
        assert_eq!(paires.len(), avant, "cle de reglage dupliquee");
    }

    #[test]
    fn tous_les_journaux_decrivent_leur_contenu() {
        // Le libelle sert de topic au salon : un salon sans description oblige
        // a deviner ce qu'il recoit.
        for j in JOURNAUX {
            assert!(!j.quoi.trim().is_empty(), "{} sans description", j.nom);
        }
    }
}
