//! Bus d'evenements de Nexus : stream `nexus:events`.
//!
//! La mecanique (consumer groups, ACK, auto-claim, deduplication) vit dans
//! `platform-common::event_bus`. Ce fichier contenait auparavant une copie
//! integrale de celle de `sentinel-bot` — 352 lignes identiques sur 353, la
//! seule difference etant la constante ci-dessous.
//!
//! Nexus ne publie pas depuis le bot (c'est `nexus-api` qui le fait, via
//! `adapters/outbound/events/redis_publisher.rs`) : seul le cote consommation
//! est expose ici.

use futures_util::Future;
use platform_common::EventBus;

pub use platform_common::default_consumer_name;

/// Nom de la stream des events Nexus.
pub const STREAM_KEY: &str = "nexus:events";

/// Le bus de cette plateforme.
const BUS: EventBus = EventBus::new(STREAM_KEY);

/// Lance un consumer durable, avec reconnexion automatique.
pub async fn listen_stream_group<F, Fut>(group: String, consumer: String, handler: F)
where
    F: Fn(String) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = ()> + Send,
{
    BUS.listen_stream_group(group, consumer, handler).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sentinel et Nexus DOIVENT rester sur deux streams distinctes : un bot
    /// qui consommerait celle de l'autre plateforme acquitterait des events
    /// qu'il ne sait pas traiter, les faisant disparaitre pour son vrai
    /// destinataire.
    #[test]
    fn stream_key_est_distincte_de_sentinel() {
        assert_eq!(STREAM_KEY, "nexus:events");
        assert_ne!(STREAM_KEY, "sentinel:events");
        assert_eq!(BUS.stream_key(), STREAM_KEY);
    }

    #[test]
    fn test_default_consumer_name_format() {
        let name = default_consumer_name();
        assert!(!name.is_empty());
    }

    #[test]
    fn stream_key_value() {
        assert_eq!(STREAM_KEY, "nexus:events");
        assert!(STREAM_KEY.contains("nexus"));
        assert!(STREAM_KEY.contains("events"));
    }

    #[test]
    fn bus_initialization() {
        assert_eq!(BUS.stream_key(), "nexus:events");
    }

    #[test]
    fn default_consumer_name_not_empty() {
        let name1 = default_consumer_name();
        let name2 = default_consumer_name();
        assert!(!name1.is_empty());
        assert!(!name2.is_empty());
    }

    #[test]
    fn stream_key_is_nexus_not_sentinel() {
        assert_ne!(STREAM_KEY, "sentinel:events");
        assert_ne!(STREAM_KEY, "atrium:events");
        assert_ne!(STREAM_KEY, "ops:events");
    }

    #[test]
    fn stream_key_matches_bus_configuration() {
        assert_eq!(STREAM_KEY, BUS.stream_key());
    }
}
