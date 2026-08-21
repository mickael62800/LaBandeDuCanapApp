//! La porte d'entree : ce qu'un membre voit AVANT d'avoir accepte le reglement.
//!
//! Le probleme qu'on ferme ici. Le salon du reglement doit etre visible par
//! les arrivants — c'est sa raison d'etre. S'il l'est aussi par les membres
//! deja valides, alors la liste de droite de CE salon affiche tout le serveur :
//! quelqu'un qui n'a rien accepte y lit les pseudos de tout le monde, et peut
//! ecrire a chacun en prive.
//!
//! La correction tient en un refus : le role du reglement (celui des membres
//! valides) ne voit plus le salon du reglement. Les arrivants, eux, le voient
//! toujours par `@everyone`. La liste ne contient alors que les autres
//! arrivants et le staff.
//!
//! CE QUE CELA NE FAIT PAS. Discord n'a aucune permission « voir les
//! membres » : la liste decoule des salons accessibles, rien d'autre. Et
//! aucun reglage de serveur n'empeche un membre d'en contacter un autre en
//! prive — partager le serveur suffit. Le SEUL mecanisme qui bloque les
//! messages prives est l'ecran de regles natif de Discord (serveur
//! Communaute), qui garde l'arrivant `pending` tant qu'il n'a pas accepte. Le
//! diagnostic le signale, mais ne peut pas l'activer : cela se fait dans les
//! parametres du serveur.

use serenity::all::{
    ChannelId, Context, GuildId, PermissionOverwrite, PermissionOverwriteType, Permissions, RoleId,
};

/// Ce qu'on a trouve en inspectant la porte.
pub struct Diagnostic {
    /// Le reglement est-il gere par le bouton de Sentinel ?
    pub bouton_actif: bool,
    pub salon_reglement: Option<ChannelId>,
    pub role_valide: Option<RoleId>,
    /// Le role des membres valides voit-il le salon du reglement ? C'est la
    /// fuite : sa presence remplit la liste des membres vue par les arrivants.
    pub valides_voient_le_salon: bool,
    /// L'ecran de regles natif de Discord est-il actif ? Lui seul empeche les
    /// messages prives.
    pub ecran_natif_actif: bool,
}

impl Diagnostic {
    /// Y a-t-il quelque chose a corriger que cette commande sache corriger ?
    pub fn verrouillage_utile(&self) -> bool {
        self.salon_reglement.is_some() && self.role_valide.is_some() && self.valides_voient_le_salon
    }
}

/// Inspecte la porte sans rien modifier.
pub async fn diagnostiquer(ctx: &Context, guild_id: GuildId) -> Option<Diagnostic> {
    let config = crate::modules::welcome::handler::load_welcome_config(ctx, guild_id).await?;
    let salon_reglement = config
        .rules_channel_id
        .as_deref()
        .and_then(|c| c.parse::<u64>().ok())
        .map(ChannelId::new);
    // `rules_role_id` accepte plusieurs roles separes par des virgules : on
    // prend le premier, celui qui ouvre le serveur.
    let role_valide = config
        .rules_role_id
        .as_deref()
        .and_then(|r| r.split(',').next())
        .map(str::trim)
        .and_then(|r| r.parse::<u64>().ok())
        .map(RoleId::new);

    // Le drapeau de guilde est la seule source fiable : un serveur peut etre
    // en mode Communaute sans avoir active l'ecran de regles.
    let ecran_natif_actif = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| {
            g.features
                .iter()
                .any(|f| f == "MEMBER_VERIFICATION_GATE_ENABLED")
        })
        .unwrap_or(false);

    let valides_voient_le_salon = match (salon_reglement, role_valide) {
        (Some(salon), Some(role)) => role_voit_le_salon(ctx, salon, role).await,
        _ => false,
    };

    Some(Diagnostic {
        bouton_actif: config.rules_enabled,
        salon_reglement,
        role_valide,
        valides_voient_le_salon,
        ecran_natif_actif,
    })
}

/// Un role voit-il ce salon ?
///
/// Un refus EXPLICITE fait foi. En son absence, le role herite de
/// `@everyone` : c'est le cas courant d'un salon de reglement ouvert, et donc
/// la fuite qu'on cherche.
async fn role_voit_le_salon(ctx: &Context, salon: ChannelId, role: RoleId) -> bool {
    let Ok(salon) = salon.to_channel(&ctx.http).await else {
        return false;
    };
    let Some(salon) = salon.guild() else {
        return false;
    };
    let refus_explicite = salon.permission_overwrites.iter().any(|o| {
        matches!(o.kind, PermissionOverwriteType::Role(r) if r == role)
            && o.deny.contains(Permissions::VIEW_CHANNEL)
    });
    !refus_explicite
}

/// Ce que devient une regle de salon quand on n'y change que le droit de VOIR.
///
/// Ecrire une regle la REMPLACE cote Discord : reconstruire naivement
/// « autorise voir » effacerait tout le reste — typiquement le refus d'ecrire
/// que porte un salon de reglement. On part donc de l'existant et on ne
/// deplace qu'un bit, d'un cote ou de l'autre.
fn regle_avec_vue(
    allow: Permissions,
    deny: Permissions,
    autorise: bool,
) -> (Permissions, Permissions) {
    if autorise {
        (
            allow | Permissions::VIEW_CHANNEL,
            deny - Permissions::VIEW_CHANNEL,
        )
    } else {
        (
            allow - Permissions::VIEW_CHANNEL,
            deny | Permissions::VIEW_CHANNEL,
        )
    }
}

/// Refuse au role des membres valides la vue du salon du reglement, et
/// s'assure que `@everyone` le voit toujours.
///
/// Les deux vont ensemble : refuser sans garantir la vue des arrivants
/// fermerait la porte a tout le monde, y compris a ceux qu'elle doit
/// accueillir.
///
/// Les autres regles du salon ne sont pas touchees — ni les refus deja poses,
/// ni les autorisations accordees a un role de moderation.
pub async fn verrouiller(
    ctx: &Context,
    salon: ChannelId,
    role_valide: RoleId,
    guild_id: GuildId,
) -> Result<(), String> {
    let everyone = RoleId::new(guild_id.get());

    // Ecrire une regle de salon REMPLACE celle qui s'y trouvait : poser un
    // simple « @everyone voit » effacerait le refus d'ecrire que la plupart
    // des salons de reglement portent, et ouvrirait le bavardage a l'entree.
    // On part donc des regles en place, et on ne touche qu'au bit de vue.
    let existant = salon
        .to_channel(&ctx.http)
        .await
        .map_err(|e| format!("lecture du salon du reglement: {e}"))?
        .guild()
        .ok_or_else(|| "le salon du reglement n'appartient a aucun serveur".to_string())?
        .permission_overwrites;

    let regle_de = |role: RoleId| -> (Permissions, Permissions) {
        existant
            .iter()
            .find(|o| matches!(o.kind, PermissionOverwriteType::Role(r) if r == role))
            .map(|o| (o.allow, o.deny))
            .unwrap_or((Permissions::empty(), Permissions::empty()))
    };

    // `@everyone` d'abord : si la seconde ecriture echoue, la porte reste
    // ouverte plutot que fermee a tous.
    let (allow_everyone, deny_everyone) = regle_de(everyone);
    let (allow_everyone, deny_everyone) = regle_avec_vue(allow_everyone, deny_everyone, true);
    salon
        .create_permission(
            &ctx.http,
            PermissionOverwrite {
                allow: allow_everyone,
                deny: deny_everyone,
                kind: PermissionOverwriteType::Role(everyone),
            },
        )
        .await
        .map_err(|e| format!("autorisation @everyone sur le salon du reglement: {e}"))?;

    let (allow_valide, deny_valide) = regle_de(role_valide);
    let (allow_valide, deny_valide) = regle_avec_vue(allow_valide, deny_valide, false);
    salon
        .create_permission(
            &ctx.http,
            PermissionOverwrite {
                allow: allow_valide,
                deny: deny_valide,
                kind: PermissionOverwriteType::Role(role_valide),
            },
        )
        .await
        .map_err(|e| format!("refus du salon au role des membres valides: {e}"))?;

    Ok(())
}

/// Annule le verrouillage : le role des membres valides revoit le salon.
///
/// Sans cette porte de sortie, une commande qui reecrit des permissions
/// Discord serait un aller simple.
pub async fn deverrouiller(
    ctx: &Context,
    salon: ChannelId,
    role_valide: RoleId,
) -> Result<(), String> {
    let existant = salon
        .to_channel(&ctx.http)
        .await
        .map_err(|e| format!("lecture du salon du reglement: {e}"))?
        .guild()
        .ok_or_else(|| "le salon du reglement n'appartient a aucun serveur".to_string())?
        .permission_overwrites;

    let Some(regle) = existant
        .iter()
        .find(|o| matches!(o.kind, PermissionOverwriteType::Role(r) if r == role_valide))
    else {
        // Rien a defaire.
        return Ok(());
    };

    let allow = regle.allow;
    let deny = regle.deny - Permissions::VIEW_CHANNEL;

    // Une regle devenue vide n'a plus de raison d'encombrer le salon ; mais
    // tant qu'elle porte autre chose, on se contente de rendre la vue —
    // supprimer l'entiere effacerait des reglages qui ne nous regardent pas.
    if allow.is_empty() && deny.is_empty() {
        return salon
            .delete_permission(&ctx.http, PermissionOverwriteType::Role(role_valide))
            .await
            .map_err(|e| format!("retrait de la regle sur le salon du reglement: {e}"));
    }

    salon
        .create_permission(
            &ctx.http,
            PermissionOverwrite {
                allow,
                deny,
                kind: PermissionOverwriteType::Role(role_valide),
            },
        )
        .await
        .map_err(|e| format!("retrait du refus sur le salon du reglement: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoriser_la_vue_ne_touche_a_aucun_autre_droit() {
        // Cas reel : un salon de reglement refuse l'ecriture a @everyone. Le
        // verrouillage doit lui rendre la vue SANS lui rendre la parole.
        let (allow, deny) =
            regle_avec_vue(Permissions::ADD_REACTIONS, Permissions::SEND_MESSAGES, true);
        assert!(allow.contains(Permissions::VIEW_CHANNEL));
        assert!(allow.contains(Permissions::ADD_REACTIONS));
        assert!(deny.contains(Permissions::SEND_MESSAGES));
        assert!(!deny.contains(Permissions::VIEW_CHANNEL));
    }

    #[test]
    fn refuser_la_vue_ne_touche_a_aucun_autre_droit() {
        let (allow, deny) = regle_avec_vue(
            Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
            Permissions::ADD_REACTIONS,
            false,
        );
        assert!(!allow.contains(Permissions::VIEW_CHANNEL));
        assert!(allow.contains(Permissions::SEND_MESSAGES));
        assert!(deny.contains(Permissions::VIEW_CHANNEL));
        assert!(deny.contains(Permissions::ADD_REACTIONS));
    }

    #[test]
    fn le_bit_de_vue_ne_reste_jamais_des_deux_cotes() {
        // Discord ferait n'importe quoi d'une regle qui autorise et refuse la
        // meme chose.
        for autorise in [true, false] {
            let (allow, deny) = regle_avec_vue(
                Permissions::VIEW_CHANNEL,
                Permissions::VIEW_CHANNEL,
                autorise,
            );
            assert!(
                !(allow.contains(Permissions::VIEW_CHANNEL)
                    && deny.contains(Permissions::VIEW_CHANNEL))
            );
        }
    }
}
