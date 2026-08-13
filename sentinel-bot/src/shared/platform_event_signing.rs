//! Signature HMAC des events INTER-PLATEFORMES publies vers Atrium.
//!
//! # Pourquoi un second module de signature
//!
//! `event_signing.rs` (a cote) signe les events destructifs de Sentinel avec
//! `SENTINEL_API_KEY`. Ce secret-la ne peut PAS servir ici : le consommateur
//! est `atrium-bot`, d'une autre plateforme. Lui donner `SENTINEL_API_KEY`
//! ouvrirait toute l'API de Sentinel — on echangerait un probleme contre un
//! pire.
//!
//! D'ou un secret dedie, `PLATFORM_EVENTS_HMAC_KEY`, distribue au producteur
//! (`sentinel-bot`) et au consommateur (`atrium-bot`), et a eux seuls. C'est la
//! meme decision que celle qui a separe `DOCKER_AGENT_TOKEN` de
//! `DOCKER_AGENT_GAME_TOKEN` : un jeton par surface, pour qu'un porteur ne
//! puisse pas deborder.
//!
//! # Ce que ca protege
//!
//! `sentinel:events` vit sur l'instance Redis COMMUNE : les trois bots, les
//! trois workers et la gateway en detiennent l'URL. Y publier ne demande aucun
//! privilege, et rien dans l'event n'atteste qu'il vient de l'AutoMod.
//!
//! Sans signature, quiconque peut ecrire dans Redis pouvait faire publier a
//! Atrium un rappel d'apaisement au moment de son choix — dans un vrai salon,
//! au nom du bot — et declencher un appel paye a DeepSeek. Meme chose pour
//! l'accueil : un event forge fait accueillir n'importe quel membre a
//! n'importe quel moment.
//!
//! # Miroir
//!
//! `atrium-bot/src/platform_event_signing.rs` porte la copie exacte. Le message
//! canonique est duplique VOLONTAIREMENT : c'est un contrat entre deux
//! processus de deux plateformes, et aucun des deux crates ne peut dependre de
//! l'autre. Modifier un `*_message` d'un seul cote fait rejeter l'event — le
//! sens de defaillance est le bon (rien n'est publie), mais ca ne se voit que
//! dans les logs du consommateur.

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

/// Secret partage entre plateformes. Vide en dev : la signature n'est alors ni
/// posee ni exigee, comme pour `event_signing`. Le compose l'exige en `:?`,
/// donc ce cas ne se produit qu'en execution locale hors Docker.
pub fn secret() -> String {
    std::env::var("PLATFORM_EVENTS_HMAC_KEY").unwrap_or_default()
}

/// Message canonique de `atrium_calming_requested`.
///
/// `channel_id` est DANS le message : sans lui, un event legitime pourrait
/// etre rejoue vers un autre salon avec la meme signature. Meme raison que
/// `wipe` dans le message de restauration.
pub fn calming_message(guild_id: &str, channel_id: &str, kind: &str) -> String {
    format!("atrium_calming:{guild_id}:{channel_id}:{kind}")
}

/// Message canonique de `atrium_welcome_requested`.
///
/// Non mentionne par l'audit, qui ne visait que l'apaisement — mais l'event
/// emprunte le meme bus, sans plus de privilege, et son effet est du meme
/// ordre : une publication au nom du bot et un appel paye au modele.
pub fn welcome_message(guild_id: &str, user_id: &str) -> String {
    format!("atrium_welcome:{guild_id}:{user_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_reproductible() {
        let m = calming_message("42", "7", "tension");
        assert_eq!(sign("s3cret", &m), sign("s3cret", &m));
        assert_eq!(sign("s3cret", &m).len(), 64);
    }

    #[test]
    fn le_salon_fait_partie_du_message_signe() {
        // Sans ca, un rappel legitime se rejoue vers n'importe quel salon.
        assert_ne!(
            sign("s3cret", &calming_message("42", "7", "tension")),
            sign("s3cret", &calming_message("42", "8", "tension")),
        );
    }

    #[test]
    fn les_deux_events_ne_partagent_pas_leurs_signatures() {
        // Un prefixe distinct empeche de faire passer un accueil pour un
        // apaisement, et inversement.
        assert_ne!(
            sign("s3cret", &calming_message("42", "7", "tension")),
            sign("s3cret", &welcome_message("42", "7")),
        );
    }

    #[test]
    fn secret_vide_ne_signe_pas() {
        assert!(sign("", &welcome_message("42", "7")).is_empty());
    }
}
