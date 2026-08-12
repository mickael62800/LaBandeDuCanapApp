//! Verification HMAC des events Redis **destructifs**.
//!
//! Miroir exact de `sentinel-api/src/adapters/inbound/http/event_signing.rs`.
//! Le message canonique est duplique volontairement : c'est un contrat entre
//! deux processus, pas du code partageable — `sentinel-bot` ne depend pas de
//! `sentinel-api`, et l'inverse serait pire. Toute modification d'un
//! `*_message` doit etre faite des DEUX cotes, sinon l'event est rejete (ce qui
//! est le bon sens de defaillance : on ne detruit rien).
//!
//! # Pourquoi ces events sont signes
//!
//! `sentinel:events` vit sur l'instance Redis COMMUNE : les trois bots, les
//! trois workers et la gateway en detiennent l'URL. Publier dessus ne demande
//! aucun privilege. Or `guild_reset` deban tout le monde, et
//! `guild_backup:restore_requested` avec `wipe` supprime TOUS les salons, roles
//! et emojis du serveur avant de restaurer. La signature est ce qui distingue
//! « l'API l'a demande » de « quelqu'un a su ecrire dans Redis ».

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// HMAC-SHA256 hexadecimal de `message`. Secret vide -> chaine vide.
pub fn sign(secret: &str, message: &str) -> String {
    if secret.is_empty() {
        return String::new();
    }
    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).expect("cle HMAC");
    mac.update(message.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Secret partage bot <-> API. Vide en dev : la signature n'est alors pas
/// exigee, coherent avec l'authentification HTTP qui est levee dans ce mode.
pub fn secret() -> String {
    std::env::var("SENTINEL_API_KEY").unwrap_or_default()
}

/// Verifie la signature portee par le champ `sig` d'un event.
///
/// Renvoie `true` si le secret est vide (mode dev). Sinon exige un `sig`
/// present et egal — une signature absente est traitee comme une signature
/// fausse, sans quoi il suffirait de l'omettre pour contourner le controle.
pub fn verifie(data: &serde_json::Value, message: &str) -> bool {
    let secret = secret();
    if secret.is_empty() {
        return true;
    }
    let attendu = sign(&secret, message);
    let recu = data.get("sig").and_then(|v| v.as_str()).unwrap_or_default();
    !recu.is_empty() && egalite_temps_constant(recu.as_bytes(), attendu.as_bytes())
}

/// Comparaison sans court-circuit. Le bus n'offre pas d'oracle de temps
/// exploitable aujourd'hui, mais un `==` sur une valeur de securite est le
/// genre de detail qu'on ne remarque plus une fois qu'il est ecrit.
fn egalite_temps_constant(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
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
pub fn guild_backup_restore_message(guild_id: &str, snapshot_id: &str, wipe: bool) -> String {
    format!("guild_backup:restore:{guild_id}:{snapshot_id}:{wipe}")
}

/// Message canonique de `guild_backup:capture_requested`.
pub fn guild_backup_capture_message(guild_id: &str) -> String {
    format!("guild_backup:capture:{guild_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_reproductible() {
        let m = guild_backup_restore_message("42", "abc", true);
        assert_eq!(sign("s3cret", &m), sign("s3cret", &m));
        assert_eq!(sign("s3cret", &m).len(), 64);
    }

    #[test]
    fn wipe_fait_partie_du_message_signe() {
        assert_ne!(
            sign("s3cret", &guild_backup_restore_message("42", "abc", false)),
            sign("s3cret", &guild_backup_restore_message("42", "abc", true)),
        );
    }

    #[test]
    fn egalite_temps_constant_se_comporte_comme_eq() {
        assert!(egalite_temps_constant(b"abc", b"abc"));
        assert!(!egalite_temps_constant(b"abc", b"abd"));
        assert!(!egalite_temps_constant(b"abc", b"ab"));
        assert!(egalite_temps_constant(b"", b""));
    }

    #[test]
    fn signature_absente_vaut_signature_fausse() {
        // Sans secret configure on ne peut pas tester `verifie` (elle passe),
        // mais la regle qu'elle encode se verifie directement : une chaine vide
        // ne peut jamais egaler un HMAC de 64 caracteres.
        let attendu = sign("s3cret", &guild_backup_capture_message("42"));
        assert!(!egalite_temps_constant(b"", attendu.as_bytes()));
    }
}
