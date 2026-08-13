use crate::nexus::domain::entities::coussin::PlayerClass;
use crate::nexus::domain::errors::DomainError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct CoussinProfile {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub class: PlayerClass,
    pub level: i32,
    pub xp: i64,
    pub atk: i32,
    pub def: i32,
    pub hp_current: i32,
    pub hp_max: i32,
    pub coins: i64,
    pub stat_points: i32,
    pub title: String,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub chaos_events: i32,
}
#[derive(Debug, Clone)]
pub struct CoussinCombat {
    pub id: uuid::Uuid,
    pub guild_id: String,
    pub attacker_id: String,
    pub defender_id: String,
    pub mise: i64,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct CoussinCombatSnapshot {
    pub combat: CoussinCombat,
    pub attacker: CoussinProfile,
    pub defender: CoussinProfile,
}

#[derive(Debug, Clone)]
pub struct CoussinProgress {
    pub xp: i64,
    pub level: i32,
    pub stat_points: i32,
    pub title: String,
}

/// Un combat resolu, tel qu'on le raconte apres coup.
///
/// Distinct de `CoussinCombat`, qui decrit un combat EN COURS de negociation
/// (en attente, accepte, refuse). Ici tout est joue : il y a un vainqueur,
/// des jets de des et un recit.
#[derive(Debug, Clone)]
pub struct CoussinCombatResult {
    pub id: uuid::Uuid,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub special_attack: Option<String>,
    pub result_message: Option<String>,
    pub coins_transferred: i64,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Un pari place sur un combat.
#[derive(Debug, Clone)]
pub struct CoussinBet {
    pub id: uuid::Uuid,
    pub backed_id: String,
    pub amount: i64,
    /// `None` tant que le combat n'est pas resolu.
    pub won: Option<bool>,
    pub payout: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Une prime posee sur la tete d'un joueur.
#[derive(Debug, Clone)]
pub struct CoussinPrime {
    pub id: uuid::Uuid,
    pub target_id: String,
    pub target_name: String,
    pub placed_by_id: String,
    pub placed_by_name: String,
    pub amount: i64,
    pub claimed: bool,
    pub claimed_by_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait CoussinRepository: Send + Sync {
    /// Paris places par un joueur, les plus recents d'abord.
    async fn list_bets(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CoussinBet>, DomainError>;

    /// Primes qui concernent un joueur : celles qu'il a posees ET celles
    /// posees sur sa tete. Les separer cote appelant serait deux requetes
    /// pour une information qui se lit ensemble.
    async fn list_primes(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CoussinPrime>, DomainError>;

    /// Derniers combats RESOLUS d'un joueur, attaquant ou defenseur.
    ///
    /// Lecture qui manquait completement : le jeu ecrivait ses combats sans
    /// jamais offrir de les relire. Le recit et les jets de des etaient donc
    /// perdus des que le message Discord defilait.
    async fn list_combat_history(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CoussinCombatResult>, DomainError>;

    async fn find_profile(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoussinProfile>, DomainError>;
    /// Classement des joueurs de la guild (supervision cote web).
    /// Trie par niveau puis XP decroissants, borne par `limit`.
    async fn list_profiles(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<CoussinProfile>, DomainError>;
    async fn create_profile(&self, profile: &CoussinProfile) -> Result<(), DomainError>;
    async fn update_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: PlayerClass,
        atk: i32,
        def: i32,
        hp_max: i32,
        cooldown_minutes: i64,
    ) -> Result<(), DomainError>;
    async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: &str,
    ) -> Result<CoussinProfile, DomainError>;
    async fn create_combat(
        &self,
        guild_id: &str,
        channel_id: &str,
        attacker: &CoussinProfile,
        defender: &CoussinProfile,
        mise: i64,
        cooldown_minutes: i64,
    ) -> Result<CoussinCombat, DomainError>;
    async fn accept_combat(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError>;
    async fn refuse_combat(&self, id: uuid::Uuid, defender_id: &str) -> Result<bool, DomainError>;
    async fn resolution_snapshot(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<CoussinCombatSnapshot>, DomainError>;
    /// Regle une bagarre : transfert de la mise, primes, paris, statistiques.
    ///
    /// `bet_payout_pct` est le gain d'un pari gagnant en pourcentage de la
    /// mise (200 = double). Il est PASSE par le cas d'usage : la valeur etait
    /// ecrite dans la requete SQL, ce qui rendait tout reglage impossible.
    #[allow(clippy::too_many_arguments)]
    async fn resolve_combat(
        &self,
        id: uuid::Uuid,
        winner_id: Option<&str>,
        attacker_roll: i32,
        defender_roll: i32,
        transferred: i64,
        attacker_hp: i32,
        defender_hp: i32,
        bet_payout_pct: i64,
        attacker_progress: Option<CoussinProgress>,
        defender_progress: Option<CoussinProgress>,
    ) -> Result<bool, DomainError>;
}
