//! Cas d'usage des hauts faits.
//!
//! Regles portees ici (cf. DOC/Nexus/haut-faits.md) :
//!
//!  - une identite de jeu NON VERIFIEE ne debloque rien ;
//!  - un haut fait `manual` n'est jamais attribue par un evenement ;
//!  - un haut fait desactive n'est jamais attribue ;
//!  - l'attribution est idempotente (deja possede / evenement deja consomme) ;
//!  - les hauts faits sont propres a une guilde : `guild_id` vient toujours de
//!    l'appelant de confiance, jamais du contenu de l'evenement de jeu.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use crate::nexus::domain::entities::achievement::{
    Achievement, AchievementProgress, GameIdentity, GamePlayerLink, UserAchievement, Verification,
};
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::achievements::{
    GameUnlockCommand, ManageAchievementsUseCase, UnlockOutcome, UnlockedAchievement,
};
use crate::nexus::ports::outbound::achievement_repository::{
    AchievementRepository, AchievementUpdate,
};

pub struct AchievementsService {
    pub repo: Arc<dyn AchievementRepository>,
}

impl AchievementsService {
    pub fn new(repo: Arc<dyn AchievementRepository>) -> Self {
        Self { repo }
    }

    /// Ecrit l'attribution et compose le resultat. Renvoie `AlreadyOwned`
    /// quand le repository n'a rien insere (contrainte d'unicite) : l'appelant
    /// ne publie alors aucune annonce.
    async fn persist_unlock(
        &self,
        achievement: Achievement,
        guild_id: &str,
        discord_user_id: &str,
        game_player_id: Option<String>,
        source_event_id: Option<String>,
        granted_by: Option<String>,
    ) -> Result<UnlockOutcome, DomainError> {
        let unlock = UserAchievement {
            id: Uuid::new_v4(),
            guild_id: guild_id.to_owned(),
            discord_user_id: discord_user_id.to_owned(),
            achievement_id: achievement.id,
            game_player_id: game_player_id.clone(),
            source_event_id: source_event_id.clone(),
            granted_by,
            unlocked_at: Utc::now(),
        };

        match self.repo.insert_unlock(&unlock).await? {
            None => Ok(UnlockOutcome::AlreadyOwned),
            Some(_) => {
                info!(
                    guild_id,
                    code = %achievement.code,
                    "haut fait attribue"
                );
                Ok(UnlockOutcome::Unlocked(Box::new(UnlockedAchievement {
                    achievement,
                    guild_id: guild_id.to_owned(),
                    discord_user_id: discord_user_id.to_owned(),
                    game_player_id,
                    source_event_id,
                })))
            }
        }
    }
}

#[async_trait]
impl ManageAchievementsUseCase for AchievementsService {
    async fn list_definitions(&self, game: Option<&str>) -> Result<Vec<Achievement>, DomainError> {
        self.repo.list_definitions(game).await
    }

    async fn update_definition(
        &self,
        id: Uuid,
        update: AchievementUpdate,
    ) -> Result<Achievement, DomainError> {
        if update.is_empty() {
            return Err(DomainError::ValidationError(
                "aucun champ a mettre a jour".into(),
            ));
        }
        if let Some(Some(url)) = update.icon_url.as_ref() {
            validate_icon_url(url)?;
        }
        if let Some(name) = update.name.as_ref() {
            if name.trim().is_empty() || name.chars().count() > 100 {
                return Err(DomainError::ValidationError(
                    "nom invalide : 1 a 100 caracteres".into(),
                ));
            }
        }
        if let Some(description) = update.description.as_ref() {
            if description.chars().count() > 500 {
                return Err(DomainError::ValidationError(
                    "description trop longue : 500 caracteres maximum".into(),
                ));
            }
        }
        self.repo.update_definition(id, update).await
    }

    async fn member_progress(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: Option<&str>,
    ) -> Result<Vec<AchievementProgress>, DomainError> {
        let definitions = self.repo.list_definitions(game).await?;
        let unlocked = self.repo.list_for_member(guild_id, discord_user_id).await?;

        Ok(definitions
            .into_iter()
            .filter_map(|achievement| {
                let unlocked_at = unlocked
                    .iter()
                    .find(|u| u.achievement_id == achievement.id)
                    .map(|u| u.unlocked_at);
                // Un haut fait secret non debloque ne doit pas fuiter dans la
                // liste : il n'apparait qu'une fois obtenu.
                if achievement.hidden && unlocked_at.is_none() {
                    return None;
                }
                // Un haut fait desactive reste visible s'il a ete obtenu (on ne
                // retire pas un acquis), mais ne se propose plus.
                if !achievement.enabled && unlocked_at.is_none() {
                    return None;
                }
                Some(AchievementProgress {
                    achievement,
                    unlocked_at,
                })
            })
            .collect())
    }

    async fn link_identity(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
        game_player_id: &str,
    ) -> Result<GamePlayerLink, DomainError> {
        // La validation du format appartient au domaine : pour Palworld, un
        // pseudo n'est pas une identite recevable.
        let identity = GameIdentity::parse(game, game_player_id)?;

        // Refus explicite si un AUTRE membre a deja revendique cette identite.
        // Le repository porte aussi la contrainte, mais le message d'erreur
        // rendu ici est comprehensible par l'utilisateur.
        if let Some(existing) = self.repo.find_link_by_player(guild_id, &identity).await? {
            if existing.discord_user_id != discord_user_id {
                return Err(DomainError::Conflict(
                    "cette identite de jeu est deja liee a un autre membre".into(),
                ));
            }
        }

        // `verified = true` : la liaison est faite par le membre lui-meme,
        // depuis son propre compte Discord. C'est la verification exigee par le
        // document — un identifiant lu dans un log n'y donnerait pas droit.
        self.repo
            .upsert_link(guild_id, discord_user_id, &identity, true)
            .await
    }

    async fn find_link(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
    ) -> Result<Option<GamePlayerLink>, DomainError> {
        self.repo.find_link(guild_id, discord_user_id, game).await
    }

    async fn unlink_identity(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        game: &str,
    ) -> Result<bool, DomainError> {
        self.repo.delete_link(guild_id, discord_user_id, game).await
    }

    async fn unlock_from_game_event(
        &self,
        cmd: GameUnlockCommand,
    ) -> Result<UnlockOutcome, DomainError> {
        let identity = GameIdentity::parse(&cmd.game, &cmd.game_player_id)?;

        // 1. Identite -> membre Discord. SANS LIAISON VERIFIEE, RIEN.
        let link = self
            .repo
            .find_link_by_player(&cmd.guild_id, &identity)
            .await?
            .filter(|l| l.is_verified())
            .ok_or_else(|| {
                DomainError::NotFound(
                    "aucune identite de jeu verifiee pour ce joueur : haut fait non attribue"
                        .into(),
                )
            })?;

        // 2. Definition du haut fait, propre au jeu.
        let achievement = self
            .repo
            .find_definition_by_code(Some(identity.game()), &cmd.achievement_code)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("haut fait inconnu : {}", cmd.achievement_code))
            })?;

        if !achievement.enabled {
            return Err(DomainError::Conflict(
                "haut fait desactive pour ce serveur".into(),
            ));
        }
        // 3. Un haut fait a validation humaine ne peut pas etre debloque par un
        // evenement, meme bien forme : c'est ce qui empeche un adaptateur de
        // jeu d'attribuer les hauts faits invérifiables.
        if achievement.verification == Verification::Manual {
            return Err(DomainError::Forbidden(
                "ce haut fait exige une validation d'administrateur".into(),
            ));
        }

        self.persist_unlock(
            achievement,
            &cmd.guild_id,
            &link.discord_user_id,
            Some(identity.player_id().to_owned()),
            Some(cmd.source_event_id),
            None,
        )
        .await
    }

    async fn grant_manually(
        &self,
        guild_id: &str,
        discord_user_id: &str,
        achievement_id: Uuid,
        granted_by: &str,
    ) -> Result<UnlockOutcome, DomainError> {
        let achievement = self
            .repo
            .find_definition(achievement_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("haut fait introuvable".into()))?;

        if !achievement.enabled {
            return Err(DomainError::Conflict(
                "haut fait desactive pour ce serveur".into(),
            ));
        }

        // Si le haut fait est propre a un jeu, on trace l'identite du membre
        // quand elle existe — sans l'exiger : une attribution manuelle est
        // justement la voie ouverte quand aucun adaptateur ne peut prouver.
        let game_player_id = match achievement.game.as_deref() {
            Some(game) => self
                .repo
                .find_link(guild_id, discord_user_id, game)
                .await?
                .filter(|l| l.is_verified())
                .map(|l| l.game_player_id),
            None => None,
        };

        self.persist_unlock(
            achievement,
            guild_id,
            discord_user_id,
            game_player_id,
            None,
            Some(granted_by.to_owned()),
        )
        .await
    }
}

/// Une image de haut fait est affichee par Discord et par le dashboard.
///
/// Deux formes sont acceptees :
///
///  - une URL absolue `http(s)://…` (image hebergee ailleurs) ;
///  - un chemin RELATIF a la racine du site (`/Achievement/palworld/pal_01.jpg`),
///    qui est la forme des images livrees avec le dashboard. On la prefere pour
///    ces dernieres parce qu'elle reste stable d'un deploiement a l'autre : une
///    URL d'asset hachee changerait a chaque build et invaliderait les images
///    deja enregistrees en base. Le bot la rend absolue avec `WEB_FRONT_URL`
///    avant de la donner a Discord.
///
/// Tout autre schema (`javascript:`, `data:`) est refuse : il n'a rien a faire
/// dans un embed ni dans une balise `<img>`. Les chemins protocol-relatifs
/// (`//hote/...`) et la remontee de repertoire (`..`) le sont aussi.
fn validate_icon_url(url: &str) -> Result<(), DomainError> {
    let url = url.trim();
    if url.is_empty() {
        return Ok(());
    }
    if url.len() > 500 {
        return Err(DomainError::ValidationError(
            "URL d'image trop longue : 500 caracteres maximum".into(),
        ));
    }
    if url.starts_with("https://") || url.starts_with("http://") {
        return Ok(());
    }
    // `//hote/x` serait interprete par le navigateur comme une URL absolue vers
    // un autre hote : ce n'est pas un chemin local malgre son apparence.
    if url.starts_with('/') && !url.starts_with("//") && !url.contains("..") {
        return Ok(());
    }
    Err(DomainError::ValidationError(
        "URL d'image invalide : http(s):// ou chemin local commencant par / attendu".into(),
    ))
}

#[cfg(test)]
#[path = "tests/achievements_service.rs"]
mod tests;
