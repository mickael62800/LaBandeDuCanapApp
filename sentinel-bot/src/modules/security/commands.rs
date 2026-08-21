//! Slash command /security (status, history).

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption,
};

use crate::shared::discord_helpers::{has_manage_guild, reply_ephemeral_embed};
use crate::shared::embeds::{critical_embed, info_embed, success_embed};

use super::{LockdownKey, QuarantineKey, RaidDetectorKey, RecentJoinsKey, SecurityApiKey};

pub fn register() -> CreateCommand {
    CreateCommand::new("security")
        .description("Commandes du security bot")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "status",
            "Affiche l'etat actuel de la securite",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "history",
                "Affiche les derniers evenements de securite",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "limit",
                    "Nombre d'evenements a afficher (defaut: 5)",
                )
                .min_int_value(1)
                .max_int_value(25)
                .required(false),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "panic",
            "🚨 Bouton panique : verrouille TOUS les salons texte immediatement",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "calm",
            "Leve le verrouillage panique et restaure les permissions",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "porte",
                "Verifie qui peut voir les membres avant d'avoir accepte le reglement",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "action",
                    "Par defaut : simple diagnostic, sans rien modifier",
                )
                .add_string_choice("Diagnostic (ne modifie rien)", "diagnostic")
                .add_string_choice("Verrouiller la porte", "verrouiller")
                .add_string_choice("Annuler le verrouillage", "deverrouiller")
                .required(false),
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let sub = command
        .data
        .options
        .first()
        .map(|o| o.name.as_str())
        .unwrap_or("");

    match sub {
        "status" => handle_status(ctx, command).await,
        "history" => handle_history(ctx, command).await,
        "panic" => handle_panic(ctx, command).await,
        "calm" => handle_calm(ctx, command).await,
        "porte" => handle_porte(ctx, command).await,
        _ => {}
    }
}

/// 🚨 Bouton panique : verrouille immediatement tous les salons texte
/// (SEND_MESSAGES refuse pour @everyone), de facon entierement reversible
/// (permissions d'origine sauvegardees, restaurees par /calm ou a l'expiration).
async fn handle_panic(ctx: &Context, command: &CommandInteraction) {
    if !has_manage_guild(command) {
        reply_ephemeral_embed(
            ctx,
            command,
            critical_embed("Permission refusee").description(
                "La permission MANAGE_GUILD est requise pour activer le mode panique.",
            ),
        )
        .await;
        tracing::warn!(
            user = %command.user.name,
            user_id = %command.user.id,
            "Tentative /security panic sans permission"
        );
        return;
    }

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };
    // Duree persistee (le worker restaure a l'expiration) ; /calm leve avant.
    let duration = std::env::var("LOCKDOWN_DURATION_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(900);

    {
        // On tient le lock data pendant activate (comme join_handler) : activate
        // relit ctx.data (ApiClientKey) en interne, re-entrance de lecture OK.
        let data = ctx.data.read().await;
        match data.get::<LockdownKey>() {
            Some(lockdown) => lockdown.activate(ctx, guild_id, duration).await,
            None => {
                drop(data);
                reply_ephemeral_embed(
                    ctx,
                    command,
                    critical_embed("Security").description("Module lockdown indisponible."),
                )
                .await;
                return;
            }
        }
    }

    let embed = critical_embed("🚨 Mode panique activé").description(format!(
        "Tous les salons texte sont verrouillés (@everyone ne peut plus écrire).\n\
             Restauration automatique dans {}s, ou immédiate via `/security calm`.",
        duration
    ));
    reply_ephemeral_embed(ctx, command, embed).await;
    tracing::warn!(guild = %guild_id, moderator = %command.user.name, "security: mode panique active");
}

/// Leve le verrouillage panique (restaure les permissions d'origine).
async fn handle_calm(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };
    {
        let data = ctx.data.read().await;
        if let Some(lockdown) = data.get::<LockdownKey>() {
            lockdown.deactivate(ctx, guild_id).await;
        }
    }
    reply_ephemeral_embed(
        ctx,
        command,
        success_embed("🔓 Mode panique levé")
            .description("Les permissions des salons ont été restaurées."),
    )
    .await;
    tracing::info!(guild = %guild_id, moderator = %command.user.name, "security: mode panique leve");
}

async fn handle_status(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };

    let data = ctx.data.read().await;

    let recent_joins = data
        .get::<RaidDetectorKey>()
        .map(|r| r.recent_joins(guild_id))
        .unwrap_or(0);

    let lockdown_active = data
        .get::<LockdownKey>()
        .map(|l| l.is_active(guild_id))
        .unwrap_or(false);

    let quarantined_count = data
        .get::<QuarantineKey>()
        .map(|q| q.quarantined_count())
        .unwrap_or(0);

    let recent_joins_tracker = data
        .get::<RecentJoinsKey>()
        .map(|r| r.recent(guild_id).len())
        .unwrap_or(0);

    let embed = info_embed("Security — Statut")
        .field(
            "Joins recents (raid detector)",
            recent_joins.to_string(),
            true,
        )
        .field(
            "Lockdown",
            if lockdown_active { "Actif" } else { "Inactif" },
            true,
        )
        .field(
            "Utilisateurs en quarantaine",
            quarantined_count.to_string(),
            true,
        )
        .field(
            "Joins recents (tracker)",
            recent_joins_tracker.to_string(),
            true,
        );

    reply_ephemeral_embed(ctx, command, embed).await;
}

async fn handle_history(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };

    let limit = command
        .data
        .options
        .first()
        .and_then(|sub| {
            if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
                opts.iter()
                    .find(|o| o.name == "limit")
                    .and_then(|o| o.value.as_i64())
            } else {
                None
            }
        })
        .unwrap_or(5) as u32;

    let data = ctx.data.read().await;
    let sec_api = match data.get::<SecurityApiKey>() {
        Some(a) => a,
        None => return,
    };

    match sec_api.list_events(&guild_id.to_string(), limit).await {
        Ok(events) => {
            let description = if events.is_empty() {
                "Aucun evenement recent.".to_string()
            } else {
                events
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let event_type = e
                            .get("event_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("inconnu");
                        let severity = e.get("severity").and_then(|v| v.as_str()).unwrap_or("info");
                        let description =
                            e.get("description").and_then(|v| v.as_str()).unwrap_or("");
                        let desc_preview = if description.chars().count() > 80 {
                            let truncated: String = description.chars().take(80).collect();
                            format!("{}...", truncated)
                        } else {
                            description.to_string()
                        };
                        format!(
                            "{}. [{}] **{}** — {}",
                            i + 1,
                            severity,
                            event_type,
                            desc_preview
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            let embed = info_embed("Security — Historique").description(description);
            reply_ephemeral_embed(ctx, command, embed).await;
        }
        Err(e) => {
            let embed = info_embed("Security — Historique")
                .description(format!("Erreur lors de la recuperation : {}", e));
            reply_ephemeral_embed(ctx, command, embed).await;
        }
    }
}

/// `/security porte` — qui voit les membres avant d'avoir accepte le reglement.
///
/// Sans argument, la commande REGARDE et ne touche a rien : une commande qui
/// reecrit des permissions Discord doit demander avant, pas apres.
async fn handle_porte(ctx: &Context, command: &CommandInteraction) {
    if !has_manage_guild(command) {
        reply_ephemeral_embed(
            ctx,
            command,
            critical_embed("Permission refusee")
                .description("La permission MANAGE_GUILD est requise pour toucher a la porte."),
        )
        .await;
        return;
    }
    let Some(guild_id) = command.guild_id else {
        return;
    };

    let action = command
        .data
        .options
        .first()
        .and_then(|o| match &o.value {
            CommandDataOptionValue::SubCommand(opts) => opts.first(),
            _ => None,
        })
        .and_then(|o| o.value.as_str())
        .unwrap_or("diagnostic");

    let Some(diag) = super::porte::diagnostiquer(ctx, guild_id).await else {
        reply_ephemeral_embed(
            ctx,
            command,
            critical_embed("Porte d'entree")
                .description("Configuration d'accueil illisible : l'API ne repond pas."),
        )
        .await;
        return;
    };

    match action {
        "verrouiller" => {
            let (Some(salon), Some(role)) = (diag.salon_reglement, diag.role_valide) else {
                reply_ephemeral_embed(
                    ctx,
                    command,
                    critical_embed("Rien a verrouiller").description(
                        "Il faut un salon de reglement ET un role attribue a l'acceptation. \
                         Configurez-les dans le module Accueil avant de reessayer.",
                    ),
                )
                .await;
                return;
            };
            match super::porte::verrouiller(ctx, salon, role, guild_id).await {
                Ok(()) => {
                    reply_ephemeral_embed(
                        ctx,
                        command,
                        success_embed("Porte verrouillee").description(format!(
                            "Le role des membres valides ne voit plus <#{salon}>.\n\n\
                             Les arrivants continuent de le voir, et n'y trouvent donc plus que \
                             le staff et les autres personnes en attente.\n\n\
                             **Ce que cela ne fait pas :** les messages prives restent possibles. \
                             Seul l'ecran de regles natif de Discord les bloque{}.\n\n\
                             Reversible par `/security porte` → Annuler le verrouillage.",
                            if diag.ecran_natif_actif {
                                " — il est actif sur ce serveur"
                            } else {
                                ", et il est INACTIF ici (parametres du serveur → Communaute)"
                            }
                        )),
                    )
                    .await;
                    tracing::info!(guild = %guild_id, salon = %salon, "porte d'entree verrouillee");
                }
                Err(error) => {
                    reply_ephemeral_embed(
                        ctx,
                        command,
                        critical_embed("Verrouillage impossible").description(format!(
                            "{error}\n\nVerifiez que le bot a la permission de gerer les salons."
                        )),
                    )
                    .await;
                }
            }
        }
        "deverrouiller" => {
            let (Some(salon), Some(role)) = (diag.salon_reglement, diag.role_valide) else {
                return;
            };
            match super::porte::deverrouiller(ctx, salon, role).await {
                Ok(()) => {
                    reply_ephemeral_embed(
                        ctx,
                        command,
                        success_embed("Verrouillage annule")
                            .description(format!("Le role des membres valides revoit <#{salon}>.")),
                    )
                    .await;
                }
                Err(error) => {
                    reply_ephemeral_embed(
                        ctx,
                        command,
                        critical_embed("Annulation impossible").description(error),
                    )
                    .await;
                }
            }
        }
        _ => {
            let mut lignes = Vec::new();
            lignes.push(format!(
                "**Salon du reglement :** {}",
                match diag.salon_reglement {
                    Some(c) => format!("<#{c}>"),
                    None => "aucun configure".to_string(),
                }
            ));
            lignes.push(format!(
                "**Role donne a l'acceptation :** {}",
                match diag.role_valide {
                    Some(r) => format!("<@&{r}>"),
                    None => "aucun configure".to_string(),
                }
            ));
            lignes.push(format!(
                "**Bouton Sentinel :** {}",
                if diag.bouton_actif {
                    "actif"
                } else {
                    "inactif"
                }
            ));
            lignes.push(String::new());
            lignes.push(format!(
                "**Liste des membres :** {}",
                if diag.valides_voient_le_salon {
                    "⚠️ les membres valides voient le salon du reglement, donc un arrivant \
                     y lit la liste de TOUT le serveur et peut ecrire a chacun."
                } else {
                    "✅ un arrivant n'y voit que le staff et les autres personnes en attente."
                }
            ));
            lignes.push(format!(
                "**Messages prives :** {}",
                if diag.ecran_natif_actif {
                    "✅ l'ecran de regles natif de Discord est actif : tant qu'un arrivant \
                     n'a pas accepte, ses messages prives vers les membres echouent."
                } else {
                    "⚠️ rien ne les empeche. Aucune permission Discord ne bloque un message \
                     prive entre membres d'un meme serveur ; seul l'ecran de regles natif \
                     (Parametres du serveur → Communaute) le fait."
                }
            ));
            if diag.verrouillage_utile() {
                lignes.push(String::new());
                lignes.push(
                    "Relancez avec **Verrouiller la porte** pour corriger le premier point."
                        .to_string(),
                );
            }

            reply_ephemeral_embed(
                ctx,
                command,
                info_embed("\u{1f6aa} Porte d'entree").description(lignes.join("\n")),
            )
            .await;
        }
    }
}
