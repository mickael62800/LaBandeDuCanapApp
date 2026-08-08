use crate::{
    application::economy_config::load_coussin,
    domain::{
        entities::coussin::{max_hp, PlayerClass},
        errors::DomainError,
    },
    ports::{
        inbound::coussin_profile::{CoussinCombatUseCase, CoussinProfileUseCase},
        outbound::{
            coussin_repository::{CoussinProfile, CoussinProgress, CoussinRepository},
            system::bot_config_repository::BotConfigRepository,
        },
    },
};
use async_trait::async_trait;
use rand::Rng;
use std::sync::Arc;

pub struct CoussinService {
    repo: Arc<dyn CoussinRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
    cooldowns:
        Arc<dyn crate::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository>,
}
impl CoussinService {
    pub fn new(
        repo: Arc<dyn CoussinRepository>,
        config_repo: Arc<dyn BotConfigRepository>,
        cooldowns: Arc<
            dyn crate::ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository,
        >,
    ) -> Self {
        Self {
            repo,
            config_repo,
            cooldowns,
        }
    }

    /// Le jeu est-il ouvert sur ce serveur ?
    ///
    /// Appele par les cas d'usage qui MODIFIENT : choisir sa classe, depenser
    /// un point, lancer une bagarre. Consulter un profil ou le classement
    /// reste possible jeu ferme — sinon fermer le jeu effacerait aussi la
    /// memoire de ce qui a ete joue.
    async fn ensure_open(&self, guild_id: &str) -> Result<(), DomainError> {
        self.open_config(guild_id).await.map(|_| ())
    }

    /// La configuration du serveur, apres verification que le jeu est ouvert.
    /// Les cas d'usage qui ont besoin des DEUX evitent ainsi une double
    /// lecture — et surtout ne peuvent pas oublier la verification.
    async fn open_config(
        &self,
        guild_id: &str,
    ) -> Result<crate::application::economy_config::CoussinConfig, DomainError> {
        let cfg = load_coussin(&self.config_repo, guild_id).await?;
        cfg.ensure_enabled()?;
        Ok(cfg)
    }
}

#[async_trait]
impl CoussinProfileUseCase for CoussinService {
    async fn combat_history(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<crate::ports::outbound::coussin_repository::CoussinCombatResult>, DomainError>
    {
        // Meme borne dure que le classement : une demande absurde ne doit
        // atteindre ni la base ni la reponse HTTP.
        self.repo
            .list_combat_history(guild_id, user_id, limit.clamp(1, 50))
            .await
    }

    async fn ranking(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<CoussinProfile>, DomainError> {
        // Borne dure : protege la reponse HTTP et la base d'une demande
        // absurde (?limit=100000) venant du client.
        let limit = limit.clamp(1, 200);
        self.repo.list_profiles(guild_id, limit).await
    }

    async fn profile(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CoussinProfile, DomainError> {
        if let Some(profile) = self.repo.find_profile(guild_id, user_id).await? {
            return Ok(profile);
        }
        let class = PlayerClass::Ecraseur;
        let (atk, def) = class.base_stats();
        let hp_max = max_hp(def, class);
        let profile = CoussinProfile {
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            username: username.into(),
            class,
            level: 1,
            xp: 0,
            atk,
            def,
            hp_current: hp_max,
            hp_max,
            coins: 100,
            stat_points: 0,
            // Le titre du niveau 1, pris a la source plutot que recopie : une
            // constante en dur ici a deja survecu au changement de nom du jeu.
            title: crate::domain::entities::coussin::title_for_level(1).into(),
            total_wins: 0,
            total_losses: 0,
            total_draws: 0,
            total_stolen: 0,
            cowardice_count: 0,
            chaos_events: 0,
        };
        self.repo.create_profile(&profile).await?;
        Ok(profile)
    }

    async fn choose_class(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        class: &str,
    ) -> Result<CoussinProfile, DomainError> {
        let cfg = self.open_config(guild_id).await?;
        crate::application::economy_config::ensure_cooldown_over(
            &self.cooldowns,
            guild_id,
            user_id,
            "class",
            "tu viens de changer de place",
        )
        .await?;
        let mut profile = self.profile(guild_id, user_id, username).await?;
        let class = PlayerClass::parse(class).ok_or_else(|| {
            DomainError::Validation(
                "classe invalide : ecraseur, ressort, piegeur ou couette".into(),
            )
        })?;
        let stats = cfg.classes.stats(class);
        let (atk, def) = (stats.atk, stats.def);
        let hp_max = crate::domain::entities::coussin::max_hp_with(def, class, &cfg.classes);
        self.repo
            .update_class(
                guild_id,
                user_id,
                class,
                atk,
                def,
                hp_max,
                cfg.class_change_cooldown_minutes,
            )
            .await?;
        profile.class = class;
        profile.atk = atk;
        profile.def = def;
        profile.hp_max = hp_max;
        profile.hp_current = hp_max;
        Ok(profile)
    }

    async fn train(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        stat: &str,
    ) -> Result<CoussinProfile, DomainError> {
        self.ensure_open(guild_id).await?;
        self.profile(guild_id, user_id, username).await?;
        if !matches!(stat, "atk" | "def") {
            return Err(DomainError::Validation("stat invalide : atk ou def".into()));
        }
        self.repo.spend_stat_point(guild_id, user_id, stat).await
    }
}
#[async_trait]
impl CoussinCombatUseCase for CoussinService {
    async fn challenge(
        &self,
        guild_id: &str,
        channel_id: &str,
        attacker_id: &str,
        attacker_name: &str,
        defender_id: &str,
        defender_name: &str,
        mise: i64,
    ) -> Result<crate::ports::outbound::coussin_repository::CoussinCombat, DomainError> {
        let cfg = self.open_config(guild_id).await?;
        cfg.validate_mise(mise)?;
        crate::application::economy_config::ensure_cooldown_over(
            &self.cooldowns,
            guild_id,
            attacker_id,
            "combat",
            "tu viens de te battre",
        )
        .await?;
        if attacker_id == defender_id {
            return Err(DomainError::Validation(
                "impossible de se defier soi-meme".into(),
            ));
        }
        let attacker = self.profile(guild_id, attacker_id, attacker_name).await?;
        let defender = self.profile(guild_id, defender_id, defender_name).await?;
        if attacker.coins < mise {
            return Err(DomainError::Validation("coins insuffisants".into()));
        }
        let combat = self
            .repo
            .create_combat(
                guild_id,
                channel_id,
                &attacker,
                &defender,
                mise,
                cfg.combat_cooldown_minutes,
            )
            .await?;
        Ok(combat)
    }
    async fn accept(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError> {
        self.repo.accept_combat(id, defender_id).await
    }
    async fn refuse(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError> {
        self.repo.refuse_combat(id, defender_id).await
    }
    async fn resolve(&self, id: uuid::Uuid) -> Result<bool, DomainError> {
        let snapshot = self
            .repo
            .resolution_snapshot(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("combat {id}")))?;
        // La configuration est lue AVANT de tirer les des : une bagarre qu'on
        // ne saurait pas regler ne doit pas consommer son alea.
        let cfg = load_coussin(&self.config_repo, &snapshot.combat.guild_id).await?;
        // Les des suivent le reglage du serveur. Bornes 2..=100 : un de a une
        // face rendrait chaque manche identique, et au-dela de cent le jet
        // ecraserait toute notion de statistique.
        let faces = cfg.dice_faces.clamp(2, 100);
        let rolls = {
            let mut rng = rand::thread_rng();
            [
                (rng.gen_range(1..=faces), rng.gen_range(1..=faces)),
                (rng.gen_range(1..=faces), rng.gen_range(1..=faces)),
                (rng.gen_range(1..=faces), rng.gen_range(1..=faces)),
            ]
        };
        let result = crate::domain::entities::coussin::resolve_combat(
            snapshot.attacker.atk,
            snapshot.attacker.def,
            snapshot.attacker.class,
            snapshot.attacker.level,
            snapshot.defender.atk,
            snapshot.defender.def,
            snapshot.defender.class,
            snapshot.defender.level,
            &rolls,
            cfg.combat_rules(),
        )
        .map_err(|message| DomainError::Validation(message.into()))?;
        let winner = match result.attacker_won {
            Some(true) => Some(snapshot.attacker.user_id.as_str()),
            Some(false) => Some(snapshot.defender.user_id.as_str()),
            None => None,
        };
        let attacker_progress = if result.attacker_won.is_some() {
            let won = result.attacker_won == Some(true);
            let xp = snapshot.attacker.xp + if won { cfg.xp_winner } else { cfg.xp_loser };
            let level = crate::domain::entities::coussin::level_for_xp_capped(xp, cfg.max_level);
            let stat_points = snapshot.attacker.stat_points
                + (level - snapshot.attacker.level).max(0) * cfg.stat_points_per_level.max(0);
            Some(CoussinProgress {
                xp,
                level,
                stat_points,
                title: crate::domain::entities::coussin::title_for_level(level).into(),
            })
        } else {
            None
        };

        let defender_progress = if result.attacker_won.is_some() {
            let won = result.attacker_won == Some(false);
            let xp = snapshot.defender.xp + if won { cfg.xp_winner } else { cfg.xp_loser };
            let level = crate::domain::entities::coussin::level_for_xp_capped(xp, cfg.max_level);
            let stat_points = snapshot.defender.stat_points
                + (level - snapshot.defender.level).max(0) * cfg.stat_points_per_level.max(0);
            Some(CoussinProgress {
                xp,
                level,
                stat_points,
                title: crate::domain::entities::coussin::title_for_level(level).into(),
            })
        } else {
            None
        };

        let resolved = self
            .repo
            .resolve_combat(
                id,
                winner,
                rolls[0].0,
                rolls[0].1,
                if winner.is_some() {
                    snapshot.combat.mise
                } else {
                    0
                },
                result.attacker_hp,
                result.defender_hp,
                cfg.bet_payout_pct,
                attacker_progress,
                defender_progress,
            )
            .await?;

        Ok(resolved)
    }
}

#[cfg(test)]
#[path = "tests/coussin_service.rs"]
mod tests;
