//! Signature HMAC des events Redis **destructifs**.
//!
//! # Pourquoi
//!
//! Le bus `sentinel:events` est porte par l'instance Redis COMMUNE : les trois
//! bots, les trois workers et la gateway en detiennent l'URL. Publier dessus ne
//! demande donc aucun privilege particulier — c'est acceptable pour un event
//! d'affichage, pas pour un event qui detruit des donnees.
//!
//! Les events couverts ici declenchent, cote bot, des actions irreversibles :
//!
//! | Event | Effet |
//! |---|---|
//! | `guild_reset` | deban de tous les bannis, levee des timeouts, retrait des roles |
//! | `guild_backup:restore_requested` | avec `wipe`, suppression de TOUS les salons, roles et emojis avant restauration |
//!
//! Sans signature, il suffisait de connaitre `REDIS_URL` pour declencher l'un ou
//! l'autre. `guild_reset` etait deja signe ; la restauration ne l'etait pas,
//! alors que c'est l'operation la plus destructive du produit.
//!
//! # Contrat
//!
//! Le secret est l'`API_KEY` partagee bot <-> API. Le **message canonique** est
//! reproduit a l'identique par le consumer (`sentinel-bot/src/shared/event_signing.rs`) :
//! toute modification d'un `*_message` doit etre faite des deux cotes, sans quoi
//! l'event est rejete.
//!
//! Secret vide (mode dev sans API_KEY) -> signature vide, et le bot n'exige
//! alors pas de signature. C'est le seul mode ou la protection est levee, et il
//! coincide avec celui ou toute l'authentification HTTP l'est deja.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// HMAC-SHA256 hexadecimal de `message`. Secret vide -> chaine vide.
pub fn sign(secret: &str, message: &str) -> String {
    if secret.is_empty() {
        return String::new();
    }
    // `new_from_slice` n'echoue que pour une taille de cle invalide, or HMAC
    // accepte toute longueur : ce chemin est inatteignable.
    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).expect("cle HMAC");
    mac.update(message.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Message canonique de l'event `guild_reset`.
pub fn guild_reset_message(
    guild_id: &str,
    unban: bool,
    unmute: bool,
    remove_roles: bool,
) -> String {
    format!("guild_reset:{guild_id}:{unban}:{unmute}:{remove_roles}")
}

/// Message canonique de `guild_backup:restore_requested`.
///
/// `wipe` fait partie du message signe : sans lui, un event de restauration
/// legitime pourrait etre rejoue en basculant le drapeau a `true`, ce qui
/// transformerait une restauration en effacement complet du serveur.
pub fn guild_backup_restore_message(guild_id: &str, snapshot_id: &str, wipe: bool) -> String {
    format!("guild_backup:restore:{guild_id}:{snapshot_id}:{wipe}")
}

/// Message canonique de `guild_backup:capture_requested`.
///
/// La capture n'est pas destructive, mais elle est signee pour la meme raison
/// que le reste : un tiers capable de publier sur le bus pourrait sinon saturer
/// le quota de captures et evincer les sauvegardes reelles.
pub fn guild_backup_capture_message(guild_id: &str) -> String {
    format!("guild_backup:capture:{guild_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_vide_ne_signe_pas() {
        assert_eq!(sign("", "guild_reset:1:true:true:true"), "");
    }

    #[test]
    fn signature_stable_et_hexadecimale() {
        let a = sign("s3cret", "guild_reset:1:true:true:true");
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(a, sign("s3cret", "guild_reset:1:true:true:true"));
    }

    #[test]
    fn un_seul_champ_qui_change_change_la_signature() {
        let sans_wipe = sign("s3cret", &guild_backup_restore_message("1", "abc", false));
        let avec_wipe = sign("s3cret", &guild_backup_restore_message("1", "abc", true));
        assert_ne!(sans_wipe, avec_wipe);
    }

    #[test]
    fn les_familles_d_events_ne_se_confondent_pas() {
        // Un message de capture ne doit jamais valider un restore, meme guild.
        assert_ne!(
            guild_backup_capture_message("1"),
            guild_backup_restore_message("1", "", false)
        );
    }

    #[test]
    fn le_secret_discrimine() {
        let m = guild_reset_message("1", true, true, true);
        assert_ne!(sign("a", &m), sign("b", &m));
    }
}
