//! Restauration d'un `GuildSnapshot` dans un serveur Discord, avec REMAPPING
//! d'IDs.
//!
//! Recree la structure dans l'ordre roles -> categories -> salons (+ overwrites)
//! -> settings -> bans -> emojis -> member_roles. Chaque etape construit une
//! table `old_id -> new_id` consommee par les etapes suivantes.
//!
//! Robustesse : sequentiel, gestion d'erreur PAR ELEMENT (une creation qui
//! echoue est loggee et n'interrompt pas la restauration). Ne parallelise pas
//! (serenity gere les rate limits sur des appels sequentiels).
//!
//! Best-effort documente : l'icone du serveur et les emojis sont telecharges
//! depuis les URLs CDN du snapshot puis recrees (echec logge sans interrompre).
//! Les membres ABSENTS ne peuvent pas recevoir leurs roles.

use std::collections::HashMap;

use serenity::all::{
    AfkTimeout, ChannelId, ChannelType, Colour, Context, CreateAttachment, CreateChannel,
    DefaultMessageNotificationLevel, EditGuild, EditRole, ExplicitContentFilter, GuildId,
    PermissionOverwrite, PermissionOverwriteType, Permissions, RoleId, UserId, VerificationLevel,
};
use tracing::{info, warn};

use platform_core::sentinel::domain::entities::guild_backup::snapshot::{
    GuildSnapshot, SnapshotChannel,
};

use super::api_client::PendingRoleGrant;
use super::progress::ProgressSink;

/// Rapport de restauration (compteurs pour le feedback final).
#[derive(Debug, Default)]
pub struct RestoreReport {
    pub roles_created: usize,
    pub roles_failed: usize,
    pub categories_created: usize,
    pub channels_created: usize,
    pub channels_failed: usize,
    pub bans_applied: usize,
    pub members_updated: usize,
    pub emojis_created: usize,
    pub emojis_total: usize,
    /// Elements existants REUTILISES (mode merge, sans wipe) au lieu d'etre
    /// recrees — evite les doublons lors d'une restauration sur un serveur non vide.
    pub roles_reused: usize,
    pub channels_reused: usize,
    /// `Some(true)` = icone restauree, `Some(false)` = echec, `None` = pas d'icone.
    pub icon_restored: Option<bool>,
    pub notes: Vec<String>,
    /// Re-attributions a persister cote API : pour TOUS les membres captures
    /// (presents ET absents), la liste des NOUVEAUX role_id (remappes). Les
    /// membres absents recuperent ainsi leurs roles a leur retour (hook join).
    pub pending_grants: Vec<PendingRoleGrant>,
}

/// Parse un bitfield de permissions (chaine) vers [`Permissions`].
fn parse_permissions(bits: &str) -> Permissions {
    let raw = bits.parse::<u64>().unwrap_or(0);
    Permissions::from_bits_truncate(raw)
}

/// Traduit le `kind` textuel du snapshot vers un [`ChannelType`] serenity.
fn channel_type(kind: &str) -> ChannelType {
    match kind {
        "voice" => ChannelType::Voice,
        "forum" => ChannelType::Forum,
        "announcement" => ChannelType::News,
        "stage" => ChannelType::Stage,
        _ => ChannelType::Text,
    }
}

/// Restaure le snapshot dans `guild_id`. Renvoie un rapport de synthese.
///
/// `merge` (= restauration SANS wipe) : au lieu de recreer aveuglement (ce qui
/// DUPLIQUE tout sur un serveur non vide), on retrouve les roles/categories/
/// salons/emojis existants PAR NOM et on ne cree que ce qui manque. Le flux
/// avec wipe (`merge = false`) reste strictement inchange (creation fraiche).
pub async fn restore(
    ctx: &Context,
    guild_id: GuildId,
    snapshot: &GuildSnapshot,
    merge: bool,
    progress: &ProgressSink<'_>,
) -> RestoreReport {
    let mut report = RestoreReport::default();

    // Tables de remapping old_id -> new_id.
    let mut role_map: HashMap<String, RoleId> = HashMap::new();
    let mut channel_map: HashMap<String, ChannelId> = HashMap::new();

    // @everyone : ne pas recreer, mapper l'ancien @everyone (== ancien guild_id)
    // vers le @everyone du serveur cible.
    role_map.insert(snapshot.guild_id.clone(), guild_id.everyone_role());

    // ── Index de l'existant (mode merge uniquement) ──
    // name -> RoleId ; (name, kind, parent) -> ChannelId ; name de categorie.
    let mut existing_role_by_name: HashMap<String, RoleId> = HashMap::new();
    let mut existing_cat_by_name: HashMap<String, ChannelId> = HashMap::new();
    let mut existing_chan: HashMap<(String, ChannelType, Option<ChannelId>), ChannelId> =
        HashMap::new();
    if merge {
        if let Ok(roles) = guild_id.roles(&ctx.http).await {
            for (rid, r) in roles {
                existing_role_by_name.entry(r.name).or_insert(rid);
            }
        }
        if let Ok(channels) = guild_id.channels(&ctx.http).await {
            for (cid, ch) in channels {
                if ch.kind == ChannelType::Category {
                    existing_cat_by_name.entry(ch.name).or_insert(cid);
                } else {
                    existing_chan
                        .entry((ch.name, ch.kind, ch.parent_id))
                        .or_insert(cid);
                }
            }
        }
    }

    // ── 1. Roles ──
    progress.set("♻️ Restauration… roles").await;
    for role in &snapshot.roles {
        // Merge : reutilise un role de meme nom deja present (pas de doublon).
        if merge {
            if let Some(&rid) = existing_role_by_name.get(&role.name) {
                role_map.insert(role.old_id.clone(), rid);
                report.roles_reused += 1;
                continue;
            }
        }
        let builder = EditRole::new()
            .name(&role.name)
            .colour(Colour::new(role.color))
            .hoist(role.hoist)
            .mentionable(role.mentionable)
            .permissions(parse_permissions(&role.permissions));
        match guild_id.create_role(&ctx.http, builder).await {
            Ok(new_role) => {
                role_map.insert(role.old_id.clone(), new_role.id);
                report.roles_created += 1;
            }
            Err(e) => {
                warn!(error = %e, role = %role.name, "guild_backup: echec creation role");
                report.roles_failed += 1;
            }
        }
    }

    // ── 2. Categories ──
    progress.set("♻️ Restauration… categories").await;
    for cat in &snapshot.categories {
        // Merge : reutilise une categorie de meme nom deja presente.
        if merge {
            if let Some(&cid) = existing_cat_by_name.get(&cat.name) {
                channel_map.insert(cat.old_id.clone(), cid);
                report.channels_reused += 1;
                continue;
            }
        }
        let builder = CreateChannel::new(&cat.name).kind(ChannelType::Category);
        match guild_id.create_channel(&ctx.http, builder).await {
            Ok(ch) => {
                channel_map.insert(cat.old_id.clone(), ch.id);
                report.categories_created += 1;
            }
            Err(e) => {
                warn!(error = %e, category = %cat.name, "guild_backup: echec creation categorie");
            }
        }
    }

    // ── 3. Salons (+ overwrites) ──
    let total = snapshot.channels.len();
    for (i, chan) in snapshot.channels.iter().enumerate() {
        if i % 5 == 0 {
            progress
                .set(&format!("♻️ Restauration… salons {}/{}", i, total))
                .await;
        }
        // Merge : reutilise un salon de meme (nom, type, categorie parente).
        if merge {
            let parent_new = chan
                .parent_old_id
                .as_ref()
                .and_then(|p| channel_map.get(p).copied());
            let key = (chan.name.clone(), channel_type(&chan.kind), parent_new);
            if let Some(&cid) = existing_chan.get(&key) {
                channel_map.insert(chan.old_id.clone(), cid);
                report.channels_reused += 1;
                continue;
            }
        }
        match create_channel(ctx, guild_id, chan, &channel_map, &role_map).await {
            Some(id) => {
                channel_map.insert(chan.old_id.clone(), id);
                report.channels_created += 1;
            }
            None => report.channels_failed += 1,
        }
    }

    // ── 4. Settings ──
    progress.set("♻️ Restauration… reglages").await;
    apply_settings(ctx, guild_id, snapshot, &channel_map, &mut report).await;

    // ── 5. Bans ──
    if !snapshot.bans.is_empty() {
        progress.set("♻️ Restauration… bannissements").await;
        for ban in &snapshot.bans {
            let Ok(uid) = ban.user_id.parse::<u64>() else {
                continue;
            };
            let reason = ban.reason.clone().unwrap_or_default();
            let res = if reason.is_empty() {
                guild_id.ban(&ctx.http, UserId::new(uid), 0).await
            } else {
                guild_id
                    .ban_with_reason(&ctx.http, UserId::new(uid), 0, &reason)
                    .await
            };
            match res {
                Ok(()) => report.bans_applied += 1,
                Err(e) => warn!(error = %e, user = %ban.user_id, "guild_backup: echec ban"),
            }
        }
    }

    // ── 6. Emojis (best-effort : telecharge l'image CDN puis recree l'emoji) ──
    if !snapshot.emojis.is_empty() {
        report.emojis_total = snapshot.emojis.len();
        let total = snapshot.emojis.len();
        // Merge : noms d'emojis deja presents (pour ne pas creer de doublon).
        let existing_emoji_names: std::collections::HashSet<String> = if merge {
            guild_id
                .emojis(&ctx.http)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.name)
                .collect()
        } else {
            std::collections::HashSet::new()
        };
        // `full` : une fois la limite du serveur atteinte (erreur Discord), on
        // arrete d'essayer pour ne pas spammer l'API inutilement.
        let mut full = false;
        for (i, emoji) in snapshot.emojis.iter().enumerate() {
            if i % 3 == 0 {
                progress
                    .set(&format!("♻️ Restauration… emojis {}/{}", i, total))
                    .await;
            }
            if full {
                break;
            }
            if merge && existing_emoji_names.contains(&emoji.name) {
                continue; // deja present : pas de doublon
            }
            let Some(bytes) = download_bytes(ctx, &emoji.image_ref).await else {
                warn!(emoji = %emoji.name, url = %emoji.image_ref, "guild_backup: echec download emoji");
                continue;
            };
            // Discord attend une image en data URI base64.
            let data_uri = CreateAttachment::bytes(bytes, "emoji").to_base64();
            match guild_id
                .create_emoji(&ctx.http, &emoji.name, &data_uri)
                .await
            {
                Ok(_) => report.emojis_created += 1,
                Err(e) => {
                    let msg = e.to_string();
                    warn!(error = %e, emoji = %emoji.name, "guild_backup: echec creation emoji");
                    // Limite d'emojis atteinte : inutile de continuer.
                    if msg.contains("Maximum number of emojis") || msg.contains("30008") {
                        full = true;
                    }
                }
            }
        }
        if full {
            report
                .notes
                .push("limite d'emojis du serveur atteinte".to_string());
        }
        info!(
            guild = %guild_id,
            created = report.emojis_created,
            total = report.emojis_total,
            "guild_backup: emojis restaures"
        );
    }

    // ── 7. member_roles (TOUS les membres) ──
    //
    // Pour chaque membre capture on traduit ses old_role_id -> nouveaux RoleId.
    // On enregistre TOUJOURS la re-attribution dans `pending_grants` (persistee
    // cote API par l'appelant) afin que les membres ABSENTS recuperent leurs
    // roles a leur retour. Les membres PRESENTS sont en plus re-rolises tout de
    // suite (l'entree pending sera consommee/purgee a leur prochain join, sans
    // effet visible).
    if !snapshot.member_roles.is_empty() {
        progress.set("♻️ Restauration… roles des membres").await;
        let mut absents = 0usize;
        for (user_id, old_roles) in &snapshot.member_roles {
            let Ok(uid) = user_id.parse::<u64>() else {
                continue;
            };
            let new_roles: Vec<RoleId> = old_roles
                .iter()
                .filter_map(|old| role_map.get(old).copied())
                // Ne pas re-ajouter @everyone (implicite).
                .filter(|r| *r != guild_id.everyone_role())
                .collect();
            if new_roles.is_empty() {
                continue;
            }
            // Persistance de la re-attribution (nouveaux role_id en chaines).
            report.pending_grants.push(PendingRoleGrant {
                user_id: user_id.clone(),
                role_ids: new_roles.iter().map(|r| r.get().to_string()).collect(),
            });
            // Application immediate si le membre est present.
            match guild_id.member(&ctx.http, UserId::new(uid)).await {
                Ok(member) => match member.add_roles(&ctx.http, &new_roles).await {
                    Ok(()) => report.members_updated += 1,
                    Err(e) => {
                        warn!(error = %e, user = %user_id, "guild_backup: echec attribution roles membre")
                    }
                },
                Err(_) => absents += 1,
            }
        }
        if absents > 0 {
            report.notes.push(format!(
                "{absents} membre(s) absent(s) : roles re-attribues a leur retour"
            ));
        }
    }

    if merge && (report.roles_reused > 0 || report.channels_reused > 0) {
        report.notes.push(format!(
            "merge (sans wipe) : {} rôle(s) et {} salon(s)/catégorie(s) existants réutilisés (pas de doublon)",
            report.roles_reused, report.channels_reused
        ));
    }

    info!(
        guild = %guild_id,
        roles = report.roles_created,
        roles_reused = report.roles_reused,
        categories = report.categories_created,
        channels = report.channels_created,
        channels_reused = report.channels_reused,
        bans = report.bans_applied,
        members = report.members_updated,
        merge,
        "guild_backup: restauration terminee"
    );

    report
}

/// Cree un salon avec son type, son parent, ses attributs et ses overwrites
/// (traduits via les tables de remapping). Renvoie l'ID cree ou `None`.
async fn create_channel(
    ctx: &Context,
    guild_id: GuildId,
    chan: &SnapshotChannel,
    channel_map: &HashMap<String, ChannelId>,
    role_map: &HashMap<String, RoleId>,
) -> Option<ChannelId> {
    let kind = channel_type(&chan.kind);
    let mut builder = CreateChannel::new(&chan.name).kind(kind).nsfw(chan.nsfw);

    if let Some(parent_old) = &chan.parent_old_id {
        if let Some(new_parent) = channel_map.get(parent_old) {
            builder = builder.category(*new_parent);
        }
    }
    if let Some(topic) = &chan.topic {
        if !topic.is_empty() {
            builder = builder.topic(topic);
        }
    }
    // Slowmode : salons textuels / forum uniquement.
    if matches!(kind, ChannelType::Text | ChannelType::Forum) && chan.slowmode > 0 {
        builder = builder.rate_limit_per_user(chan.slowmode.min(u16::MAX as u32) as u16);
    }
    // Bitrate / user_limit : salons vocaux / stage uniquement.
    if matches!(kind, ChannelType::Voice | ChannelType::Stage) {
        if let Some(bitrate) = chan.bitrate {
            builder = builder.bitrate(bitrate);
        }
        if let Some(limit) = chan.user_limit {
            builder = builder.user_limit(limit);
        }
    }

    // Overwrites : traduit la cible via les tables de remapping.
    let mut overwrites: Vec<PermissionOverwrite> = Vec::new();
    for ow in &chan.overwrites {
        let kind = match ow.target_type.as_str() {
            "role" => match role_map.get(&ow.target_old_id) {
                Some(rid) => PermissionOverwriteType::Role(*rid),
                None => continue, // role non remappe (managed / disparu)
            },
            "member" => match ow.target_old_id.parse::<u64>() {
                Ok(uid) => PermissionOverwriteType::Member(UserId::new(uid)),
                Err(_) => continue,
            },
            _ => continue,
        };
        overwrites.push(PermissionOverwrite {
            allow: parse_permissions(&ow.allow),
            deny: parse_permissions(&ow.deny),
            kind,
        });
    }
    if !overwrites.is_empty() {
        builder = builder.permissions(overwrites);
    }

    match guild_id.create_channel(&ctx.http, builder).await {
        Ok(ch) => Some(ch.id),
        Err(e) => {
            warn!(error = %e, channel = %chan.name, "guild_backup: echec creation salon");
            None
        }
    }
}

/// Applique les reglages generaux (best-effort), icone du serveur incluse.
async fn apply_settings(
    ctx: &Context,
    guild_id: GuildId,
    snapshot: &GuildSnapshot,
    channel_map: &HashMap<String, ChannelId>,
    report: &mut RestoreReport,
) {
    let s = &snapshot.settings;
    let mut builder = EditGuild::new()
        .name(&s.name)
        .verification_level(VerificationLevel::from(s.verification_level as u8))
        .default_message_notifications(Some(DefaultMessageNotificationLevel::from(
            s.default_notifications as u8,
        )))
        .explicit_content_filter(Some(ExplicitContentFilter::from(
            s.explicit_content_filter as u8,
        )))
        .afk_timeout(AfkTimeout::from(s.afk_timeout as u16));

    if let Some(old) = &s.afk_channel_old_id {
        builder = builder.afk_channel(channel_map.get(old).copied());
    }
    if let Some(old) = &s.system_channel_old_id {
        builder = builder.system_channel_id(channel_map.get(old).copied());
    }

    // Icone : si presente, telecharge les bytes et applique via EditGuild::icon.
    // On l'ajoute au meme builder pour n'emettre qu'une requete edit.
    let mut icon_attachment: Option<CreateAttachment> = None;
    if let Some(icon_url) = &s.icon {
        match download_bytes(ctx, icon_url).await {
            Some(bytes) => icon_attachment = Some(CreateAttachment::bytes(bytes, "icon.png")),
            None => {
                warn!(guild = %guild_id, url = %icon_url, "guild_backup: echec download icone");
                report.icon_restored = Some(false);
                report
                    .notes
                    .push("icone non restauree (download)".to_string());
            }
        }
    }
    if let Some(att) = &icon_attachment {
        builder = builder.icon(Some(att));
    }

    // Permissions de base de @everyone.
    //
    // Edition SEPAREE : ce n'est pas un reglage de serveur mais un role, qui
    // passe par une autre route de l'API Discord.
    //
    // Une chaine vide veut dire « sauvegarde prise avant que ce champ existe » :
    // on ne touche a rien. Ecrire un bitfield nul retirerait au contraire TOUS
    // les droits a tout le monde — un serveur muet, sans que rien ne l'explique.
    if !s.everyone_permissions.is_empty() {
        match s.everyone_permissions.parse::<u64>() {
            Ok(bits) => {
                let perms = Permissions::from_bits_truncate(bits);
                if let Err(e) = guild_id
                    .edit_role(
                        &ctx.http,
                        guild_id.everyone_role(),
                        EditRole::new().permissions(perms),
                    )
                    .await
                {
                    warn!(error = %e, "guild_backup: echec permissions @everyone");
                    report
                        .notes
                        .push("permissions @everyone non restaurees".to_string());
                }
            }
            Err(_) => {
                report
                    .notes
                    .push("permissions @everyone illisibles dans la sauvegarde".to_string());
            }
        }
    }

    if let Err(e) = guild_id.edit(&ctx.http, builder).await {
        warn!(error = %e, "guild_backup: echec application des reglages");
        report
            .notes
            .push("reglages du serveur partiellement appliques".to_string());
        if icon_attachment.is_some() {
            report.icon_restored = Some(false);
        }
    } else if icon_attachment.is_some() {
        report.icon_restored = Some(true);
        info!(guild = %guild_id, "guild_backup: icone restauree");
    }
}

/// Telecharge des bytes depuis une URL (CDN Discord) via le client reqwest
/// partage du bot (pooling + timeouts coherents). Best-effort : `None` en cas
/// d'echec reseau, statut non-2xx ou absence de client.
async fn download_bytes(ctx: &Context, url: &str) -> Option<Vec<u8>> {
    let client = {
        let data = ctx.data.read().await;
        let base = data.get::<crate::shared::heartbeat::ApiClientKey>()?;
        base.client().clone()
    };
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        warn!(status = %resp.status(), url, "guild_backup: download non-success");
        return None;
    }
    resp.bytes().await.ok().map(|b| b.to_vec())
}
