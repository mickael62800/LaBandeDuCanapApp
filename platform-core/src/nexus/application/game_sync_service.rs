//! Consolidation des jeux mentionnables : constat, puis reparation choisie.
//!
//! Le service ne decide jamais du sens de la reparation. Il produit un constat
//! et applique la direction qu'un humain a designee, ecart par ecart. Voir
//! `domain::entities::casino::game_sync` pour la comparaison elle-meme.

use std::sync::Arc;

use serde_json::json;

use crate::nexus::domain::entities::casino::game::DEFAULT_GAME_ROLE_COLOR;
use crate::nexus::domain::entities::casino::game_sync::{
    build_sync_report, DiscordInventory, Divergence, StoredGame, StoredPanel, SyncDirection,
    SyncReport,
};
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::outbound::casino::game_repository::GameRepository;
use crate::nexus::ports::outbound::casino::game_sync_repository::GameSyncRepository;
use crate::nexus::ports::outbound::events::{game_events, EventPublisher};

pub struct GameSyncService {
    pub game_repo: Arc<dyn GameRepository>,
    pub sync_repo: Arc<dyn GameSyncRepository>,
    pub events: Arc<dyn EventPublisher>,
}

/// Ce qu'une resolution a reellement produit, pour que l'ecran ne mente pas :
/// une reparation Discord est demandee au bot, elle n'est pas constatee ici.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolutionOutcome {
    pub key: String,
    /// `true` si la base a ete modifiee dans cette transaction.
    pub applied_now: bool,
    /// `true` si une demande a ete envoyee au bot, dont l'effet reste a
    /// confirmer par un prochain inventaire.
    pub requested_from_discord: bool,
    pub detail: String,
}

impl GameSyncService {
    pub fn new(
        game_repo: Arc<dyn GameRepository>,
        sync_repo: Arc<dyn GameSyncRepository>,
        events: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            game_repo,
            sync_repo,
            events,
        }
    }

    /// Confronte l'etat enregistre au dernier inventaire connu.
    pub async fn report(&self, guild_id: &str) -> Result<SyncReport, DomainError> {
        let games: Vec<StoredGame> = self
            .game_repo
            .list(guild_id)
            .await?
            .into_iter()
            .map(|game| StoredGame {
                id: game.id,
                name: game.game_name,
                role_id: game.role_id,
            })
            .collect();

        let panels: Vec<StoredPanel> = self
            .game_repo
            .list_panels(guild_id)
            .await?
            .into_iter()
            .map(|panel| StoredPanel {
                id: panel.id,
                channel_id: panel.channel_id.to_string(),
                message_id: panel.message_id.to_string(),
            })
            .collect();

        let stored = self.sync_repo.latest_inventory(guild_id).await?;
        let (inventory, taken_at) = match stored {
            Some(s) => (Some(s.inventory), Some(s.taken_at)),
            None => (None, None),
        };

        Ok(build_sync_report(
            &games,
            &panels,
            inventory.as_ref(),
            taken_at,
            DEFAULT_GAME_ROLE_COLOR,
        ))
    }

    /// Enregistre l'inventaire rendu par le bot.
    pub async fn record_inventory(
        &self,
        guild_id: &str,
        inventory: &DiscordInventory,
    ) -> Result<(), DomainError> {
        self.sync_repo.save_inventory(guild_id, inventory).await
    }

    /// Demande au bot un inventaire frais. La reponse est asynchrone : l'ecran
    /// doit annoncer une verification lancee, pas une verification terminee.
    pub async fn request_inventory(&self, guild_id: &str) {
        self.events
            .publish(
                game_events::GAMES_SYNC_REQUESTED,
                json!({ "guild_id": guild_id }),
            )
            .await;
    }

    /// Le bot a vu un role disparaitre dans Discord. On coupe la liaison tout
    /// de suite : garder un identifiant mort ferait echouer chaque attribution
    /// jusqu'a la prochaine verification, sans que personne ne sache pourquoi.
    ///
    /// Le jeu, lui, n'est jamais supprime ici — c'est une decision humaine.
    pub async fn forget_vanished_role(
        &self,
        guild_id: &str,
        role_id: &str,
    ) -> Result<Vec<String>, DomainError> {
        let games = self.game_repo.list(guild_id).await?;
        let mut touched = Vec::new();
        for game in games {
            if game.role_id.as_deref() == Some(role_id) {
                self.game_repo.set_role_id(guild_id, &game.id, None).await?;
                touched.push(game.game_name);
            }
        }
        Ok(touched)
    }

    /// Applique la direction retenue pour un ecart.
    ///
    /// Le rapport est recalcule ici plutot que de faire confiance a ce que le
    /// navigateur renvoie : entre l'affichage et le clic, l'ecart a pu etre
    /// resolu autrement, et agir sur un constat perime detruirait un etat
    /// redevenu correct.
    pub async fn resolve(
        &self,
        guild_id: &str,
        key: &str,
        direction: SyncDirection,
    ) -> Result<ResolutionOutcome, DomainError> {
        let report = self.report(guild_id).await?;
        let divergence = report
            .divergences
            .into_iter()
            .find(|d| d.key() == key)
            .ok_or_else(|| {
                DomainError::NotFound(
                    "Cet ecart n'existe plus : relance une verification".to_string(),
                )
            })?;

        let outcome = match (&divergence, direction) {
            // Role disparu de Discord.
            (
                Divergence::RoleMissing {
                    game_id, game_name, ..
                },
                SyncDirection::Discord,
            ) => {
                self.game_repo.set_role_id(guild_id, game_id, None).await?;
                ResolutionOutcome {
                    key: key.to_string(),
                    applied_now: true,
                    requested_from_discord: false,
                    detail: format!("Liaison effacee pour « {game_name} »"),
                }
            }
            (Divergence::RoleMissing { game_name, .. }, SyncDirection::Dashboard)
            | (Divergence::RoleUnbound { game_name, .. }, SyncDirection::Dashboard) => {
                // Le bot recree les roles manquants et persiste la liaison.
                self.events
                    .publish(
                        game_events::GAMES_ROLES_ENSURE,
                        json!({ "guild_id": guild_id }),
                    )
                    .await;
                ResolutionOutcome {
                    key: key.to_string(),
                    applied_now: false,
                    requested_from_discord: true,
                    detail: format!("Recreation du role demandee pour « {game_name} »"),
                }
            }
            // Un jeu sans role ne peut pas s'aligner sur Discord : il n'y a
            // rien a y lire. Le refuser vaut mieux que faire semblant.
            (Divergence::RoleUnbound { .. }, SyncDirection::Discord) => {
                return Err(DomainError::ValidationError(
                    "Ce jeu n'a aucun role dans Discord : la seule issue est d'en creer un, ou de supprimer le jeu."
                        .into(),
                ));
            }
            // Role orphelin.
            (Divergence::RoleOrphan { role_name, .. }, SyncDirection::Discord) => {
                ResolutionOutcome {
                    key: key.to_string(),
                    applied_now: false,
                    requested_from_discord: false,
                    detail: format!("« {role_name} » est conserve tel quel dans Discord"),
                }
            }
            (
                Divergence::RoleOrphan {
                    role_id, role_name, ..
                },
                SyncDirection::Dashboard,
            ) => {
                self.events
                    .publish(
                        game_events::GAME_ROLE_DELETE,
                        json!({
                            "guild_id": guild_id,
                            "role_id": role_id,
                            "game_name": role_name,
                        }),
                    )
                    .await;
                ResolutionOutcome {
                    key: key.to_string(),
                    applied_now: false,
                    requested_from_discord: true,
                    detail: format!("Suppression du role « {role_name} » demandee"),
                }
            }
            // Panneau disparu.
            (Divergence::PanelMessageMissing { message_id, .. }, SyncDirection::Discord) => {
                self.game_repo.delete_panel(guild_id, message_id).await?;
                ResolutionOutcome {
                    key: key.to_string(),
                    applied_now: true,
                    requested_from_discord: false,
                    detail: "Panneau oublie cote dashboard".to_string(),
                }
            }
            (
                Divergence::PanelMessageMissing {
                    channel_id,
                    message_id,
                    ..
                },
                SyncDirection::Dashboard,
            ) => {
                // L'ancien message n'existe plus : sa trace part avec lui, sinon
                // le prochain rapport signalerait le meme ecart indefiniment.
                self.game_repo.delete_panel(guild_id, message_id).await?;
                self.events
                    .publish(
                        game_events::GAMES_PANEL_DEPLOY,
                        json!({ "guild_id": guild_id, "channel_id": channel_id }),
                    )
                    .await;
                ResolutionOutcome {
                    key: key.to_string(),
                    applied_now: true,
                    requested_from_discord: true,
                    detail: "Redeploiement du panneau demande".to_string(),
                }
            }
        };

        Ok(outcome)
    }
}
