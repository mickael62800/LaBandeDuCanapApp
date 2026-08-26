//! Adaptateur : Nexus demande une annonce, Atrium l'ecrit.
//!
//! LES DEUX DOMAINES VIVENT DANS LE MEME SERVICE, donc l'appel est en memoire :
//! ni proto, ni reseau, ni second point de panne. Le contrat reste explicite —
//! le port `GameAnnouncementGateway` cote Nexus — pour que la frontiere entre
//! les deux domaines soit lisible dans le code et non seulement dans un
//! diagramme.
//!
//! C'est ici, et nulle part ailleurs, que le `game_context` de la guilde est lu :
//! Nexus n'a pas a connaitre une cle de configuration d'Atrium.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use platform_core::atrium::domain::{GameAnnouncementError, GameAnnouncementRequest};
use platform_core::atrium::ports::inbound::GenerateGameAnnouncementUseCase;

use platform_core::nexus::ports::outbound::game::announcement_gateway::{
    AnnouncementError, GameAnnouncementGateway, SessionFacts,
};

pub struct AtriumAnnouncementGateway {
    redaction: Arc<dyn GenerateGameAnnouncementUseCase>,
    /// Base de configuration d'Atrium, ou vit `game_context`.
    config_pool: Option<PgPool>,
}

impl AtriumAnnouncementGateway {
    pub fn new(
        redaction: Arc<dyn GenerateGameAnnouncementUseCase>,
        config_pool: Option<PgPool>,
    ) -> Self {
        Self {
            redaction,
            config_pool,
        }
    }

    /// Consigne de ton de la guilde.
    ///
    /// Une lecture ratee vaut « pas de consigne », pas un echec : l'annonce
    /// sortira avec le ton par defaut. Refuser d'ecrire parce qu'une table de
    /// configuration est momentanement illisible bloquerait l'ouverture d'une
    /// soiree pour une raison sans rapport avec elle.
    async fn contexte(&self, guild_id: &str) -> String {
        let Some(pool) = self.config_pool.as_ref() else {
            return String::new();
        };
        match crate::atrium::guild_config::load(pool, guild_id).await {
            Ok(config) => config.get("game_context").cloned().unwrap_or_default(),
            Err(error) => {
                tracing::warn!(%error, guild_id, "contexte de jeu illisible, ton par defaut");
                String::new()
            }
        }
    }
}

#[async_trait]
impl GameAnnouncementGateway for AtriumAnnouncementGateway {
    async fn rediger(&self, faits: SessionFacts) -> Result<String, AnnouncementError> {
        let demande = GameAnnouncementRequest {
            admin_context: self.contexte(&faits.guild_id).await,
            guild_id: faits.guild_id,
            game_name: faits.game_name,
            server_name: faits.server_name,
            max_players: faits.max_players,
            opening_label: faits.opening_label,
            schedule_label: faits.schedule_label,
        };

        self.redaction
            .announce(demande)
            .await
            .map(|annonce| annonce.content)
            .map_err(|erreur| match erreur {
                // « Retente plus tard » : c'est ce que la reprise attend, et
                // c'est le seul cas ou elle doit repasser.
                GameAnnouncementError::Unavailable => AnnouncementError::Indisponible,
                // Une demande mal formee ne passera jamais, quel que soit le
                // nombre de tentatives : la signaler comme indisponible ferait
                // boucler la reprise indefiniment.
                autre => AnnouncementError::Refusee(autre.to_string()),
            })
    }
}
