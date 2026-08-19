//! Collecte de la presence en direct pour la page membre du site.
//!
//! # La regle qui compte
//!
//! La page membre est PUBLIQUE : n'importe qui sur Internet la consulte. Or
//! « Kalyx est dans #staff » est une information privee. On ne publie donc
//! que les salons ou @everyone a le droit de voir — ce que seul le bot peut
//! determiner, l'API n'ayant aucune vue sur les permissions Discord.
//!
//! Le filtre est FERMANT : en cas de doute (salon introuvable dans le cache,
//! guilde absente), on ne publie pas. Une section vide est sans consequence ;
//! une fuite ne se rattrape pas.

use std::sync::Arc;

use serenity::model::id::GuildId;
use serenity::model::permissions::Permissions;
use serenity::prelude::*;

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;
use crate::shared::presence::{VoiceChannelDto, VoiceMemberDto};

/// Un salon est-il visible par tout le monde ?
///
/// Sur Discord, le role @everyone porte l'identifiant de la guilde. On
/// interroge les permissions de ce role sur le salon : s'il n'a pas
/// `VIEW_CHANNEL`, le salon est reserve.
///
/// Le salon reserve n'est plus ECARTE mais MARQUE : c'est l'API qui tranche
/// selon l'appelant, un membre connecte ayant de toute facon acces a ces
/// salons sur Discord. Le bot reste seul juge des permissions — l'API n'a
/// aucune vue dessus — mais il ne decide plus du public.
fn est_public(
    guild: &serenity::model::guild::Guild,
    channel_id: serenity::model::id::ChannelId,
) -> bool {
    let Some(channel) = guild.channels.get(&channel_id) else {
        // Salon inconnu du cache : on s'abstient plutot que de supposer.
        return false;
    };

    let everyone = serenity::model::id::RoleId::new(guild.id.get());
    let Some(role) = guild.roles.get(&everyone) else {
        return false;
    };

    // Permissions de base du role, puis application des surcharges du salon.
    let mut permissions = role.permissions;
    for surcharge in &channel.permission_overwrites {
        if let serenity::model::channel::PermissionOverwriteType::Role(id) = surcharge.kind {
            if id == everyone {
                permissions = (permissions & !surcharge.deny) | surcharge.allow;
            }
        }
    }

    permissions.contains(Permissions::VIEW_CHANNEL)
}

/// Reconstruit l'instantane vocal complet d'une guilde depuis le cache.
///
/// Instantane complet et non delta : appliquer des deltas supposerait qu'aucun
/// evenement ne se perde, et un seul manque ferait deriver la liste sans
/// jamais se corriger.
fn instantane(guild: &serenity::model::guild::Guild) -> Vec<VoiceChannelDto> {
    let mut par_salon: std::collections::HashMap<
        serenity::model::id::ChannelId,
        Vec<VoiceMemberDto>,
    > = std::collections::HashMap::new();

    for etat in guild.voice_states.values() {
        let Some(channel_id) = etat.channel_id else {
            continue;
        };
        // Les bots occupent les salons sans y participer : un lecteur de
        // musique afficherait un faux participant.
        //
        // Un membre ABSENT du cache n'est pas ecarte : ce serait le faire
        // disparaitre du salon alors qu'il y est bel et bien. Le cas est
        // rare (l'intent GUILD_MEMBERS le remplit) mais il ne doit pas
        // aboutir a une liste fausse — mieux vaut un nom generique qu'un
        // absent.
        let membre = guild.members.get(&etat.user_id);
        // L'etat vocal porte souvent le membre : le consulter d'abord ecarte
        // les bots que le cache des membres ne connait pas encore.
        let est_bot = etat
            .member
            .as_ref()
            .map(|m| m.user.bot)
            .or_else(|| membre.map(|m| m.user.bot))
            .unwrap_or(false);
        if est_bot {
            continue;
        }

        let nom = membre
            .map(|m| {
                m.nick
                    .clone()
                    .or_else(|| m.user.global_name.clone())
                    .unwrap_or_else(|| m.user.name.clone())
            })
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "Un membre".to_string());

        par_salon
            .entry(channel_id)
            .or_default()
            .push(VoiceMemberDto {
                user_id: etat.user_id.to_string(),
                username: nom,
                self_mute: etat.self_mute,
                self_deaf: etat.self_deaf,
                server_mute: etat.mute,
                streaming: etat.self_stream.unwrap_or(false),
                video: etat.self_video,
            });
    }

    par_salon
        .into_iter()
        .filter_map(|(channel_id, members)| {
            let channel = guild.channels.get(&channel_id)?;
            Some(VoiceChannelDto {
                channel_id: channel_id.to_string(),
                channel_name: channel.name.clone(),
                members,
                restreint: !est_public(guild, channel_id),
            })
        })
        .collect()
}

/// A appeler sur chaque changement d'etat vocal.
pub async fn on_voice_state_update(ctx: &Context, guild_id: GuildId) {
    let Some(api) = client_api(ctx).await else {
        return;
    };

    // Le cache doit etre lu de facon synchrone : garder une reference a la
    // guilde a travers un `await` bloquerait le cache pour tout le bot.
    let channels = {
        let Some(guild) = ctx.cache.guild(guild_id) else {
            return;
        };
        instantane(&guild)
    };

    api.publish_voice_presence(&guild_id.to_string(), channels);
}

/// A appeler sur chaque message. Enregistre une prise de parole.
pub async fn on_message(ctx: &Context, msg: &serenity::model::channel::Message) {
    let Some(guild_id) = msg.guild_id else {
        return;
    };
    if msg.author.bot {
        return;
    }

    let Some(api) = client_api(ctx).await else {
        return;
    };

    // Bloc synchrone : garder une reference au cache a travers un `await`
    // le bloquerait pour tout le bot.
    let nom_salon = {
        let Some(guild) = ctx.cache.guild(guild_id) else {
            return;
        };
        if !est_public(&guild, msg.channel_id) {
            return;
        }
        match guild.channels.get(&msg.channel_id) {
            Some(c) => c.name.clone(),
            None => return,
        }
    };

    let auteur = msg
        .author_nick(&ctx.http)
        .await
        .or_else(|| msg.author.global_name.clone())
        .unwrap_or_else(|| msg.author.name.clone());

    api.touch_text_presence(
        &guild_id.to_string(),
        &msg.channel_id.to_string(),
        &nom_salon,
        &auteur,
    );
}

/// Republie la presence de toutes les guildes connues.
///
/// A appeler au demarrage : sans cela, les personnes DEJA en vocal quand le
/// bot se lance n'apparaissent qu'a leur premier mouvement — c'est-a-dire
/// souvent jamais pendant toute une soiree.
pub async fn republier_tout(ctx: &Context) {
    let Some(api) = client_api(ctx).await else {
        return;
    };

    for guild_id in ctx.cache.guilds() {
        let channels = {
            let Some(guild) = ctx.cache.guild(guild_id) else {
                continue;
            };
            instantane(&guild)
        };
        api.publish_voice_presence(&guild_id.to_string(), channels);
    }
}

/// Republie la presence a intervalle regulier.
///
/// Indispensable, et pas un simple confort : l'API considere un instantane
/// perime au-dela de trois minutes. Publier uniquement sur changement d'etat
/// faisait donc disparaitre de la page tout un salon ou personne ne bougeait
/// — c'est-a-dire un salon ou l'on discute tranquillement, exactement ce
/// qu'on veut montrer.
///
/// L'intervalle doit rester nettement sous ce seuil pour absorber un rate
/// limit ou une coupure Redis passagere.
pub fn spawn_background(ctx: Context) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    tokio::spawn(async move {
        // Premiere publication tout de suite : le cache est deja rempli quand
        // `ready` se declenche, et attendre une minute laisserait la page
        // vide juste apres un redemarrage.
        republier_tout(&ctx).await;

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            republier_tout(&ctx).await;
        }
    });
}

async fn client_api(ctx: &Context) -> Option<Arc<BaseApiClient>> {
    let data = ctx.data.read().await;
    data.get::<ApiClientKey>().cloned()
}
