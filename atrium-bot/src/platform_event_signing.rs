//! Verification HMAC des events recus de Sentinel.
//!
//! Miroir exact de `sentinel-bot/src/shared/platform_event_signing.rs`. Le
//! message canonique est duplique VOLONTAIREMENT : c'est un contrat entre deux
//! processus de deux plateformes, et aucun des deux crates ne peut dependre de
//! l'autre — c'est deja la regle retenue pour les events destructifs de
//! Sentinel, et elle vaut a plus forte raison entre plateformes.
//!
//! # Ce qu'on ferme
//!
//! `sentinel:events` vit sur l'instance Redis COMMUNE : les trois bots, les
//! trois workers et la gateway en detiennent l'URL. Y publier ne demande aucun
//! privilege, et rien dans l'event n'attestait qu'il venait de l'AutoMod.
//! Quiconque pouvait ecrire dans Redis faisait donc publier a Atrium — au nom
//! du bot, dans un vrai salon — un rappel d'apaisement ou un accueil, avec un
//! appel paye a DeepSeek a la cle.
//!
//! Ce que la signature ne couvre PAS : elle atteste de l'ORIGINE, pas de la
//! fraicheur. Un event legitime capture reste rejouable a l'identique. Les
//! garde-fous existants bornent l'impact de ce rejeu — cooldown Redis partage,
//! plafond de depense (`BudgetGuard`), validation du salon, mentions bornees.
//!
//! # Secret
//!
//! `PLATFORM_EVENTS_HMAC_KEY`, distribue au seul producteur et au seul
//! consommateur. PAS `SENTINEL_API_KEY` : ce secret-la ouvre toute l'API de
//! Sentinel, et le donner a une autre plateforme echangerait un probleme
//! contre un pire.

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

pub fn secret() -> String {
    std::env::var("PLATFORM_EVENTS_HMAC_KEY").unwrap_or_default()
}

/// Verifie la signature portee par le champ `sig`.
///
/// Secret vide (dev hors Docker) : accepte, comme le fait Sentinel pour ses
/// propres events. Sinon exige un `sig` present et egal — une signature absente
/// vaut une signature fausse, sans quoi il suffirait de l'omettre.
pub fn verifie(sig_recue: &str, message: &str) -> bool {
    let secret = secret();
    if secret.is_empty() {
        return true;
    }
    let attendu = sign(&secret, message);
    !sig_recue.is_empty() && egalite_temps_constant(sig_recue.as_bytes(), attendu.as_bytes())
}

/// Comparaison sans court-circuit, comme cote Sentinel.
fn egalite_temps_constant(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Message canonique de `atrium_calming_requested`. `channel_id` en fait
/// partie : sans lui, un event legitime se rejouerait vers un autre salon.
pub fn calming_message(guild_id: &str, channel_id: &str, kind: &str) -> String {
    format!("atrium_calming:{guild_id}:{channel_id}:{kind}")
}

/// Message canonique de `atrium_welcome_requested`.
pub fn welcome_message(guild_id: &str, user_id: &str) -> String {
    format!("atrium_welcome:{guild_id}:{user_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_messages_sont_identiques_a_ceux_de_sentinel() {
        // Ces deux chaines sont le CONTRAT. Les changer ici sans les changer
        // dans `sentinel-bot` fait rejeter tous les events — sens de
        // defaillance correct, mais visible seulement dans ces logs.
        assert_eq!(
            calming_message("42", "7", "tension"),
            "atrium_calming:42:7:tension"
        );
        assert_eq!(welcome_message("42", "7"), "atrium_welcome:42:7");
    }

    #[test]
    fn signature_valide_acceptee_et_signature_fausse_refusee() {
        let message = calming_message("42", "7", "tension");
        let bonne = sign("s3cret", &message);
        assert!(egalite_temps_constant(
            bonne.as_bytes(),
            sign("s3cret", &message).as_bytes()
        ));
        assert!(!egalite_temps_constant(
            bonne.as_bytes(),
            sign("autre", &message).as_bytes()
        ));
    }

    #[test]
    fn signature_absente_vaut_signature_fausse() {
        let attendu = sign("s3cret", &welcome_message("42", "7"));
        assert!(!egalite_temps_constant(b"", attendu.as_bytes()));
    }
}
