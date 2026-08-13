use async_trait::async_trait;

use crate::sentinel::domain::entities::community::guild_member::GuildMember;
use crate::sentinel::domain::entities::community::milestone::JoinAnniversary;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait MemberRepository: Send + Sync {
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<GuildMember>, DomainError>;
    async fn find_one(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<GuildMember>, DomainError>;
    async fn upsert(&self, member: &GuildMember) -> Result<(), DomainError>;
    async fn upsert_many(&self, members: &[GuildMember]) -> Result<u64, DomainError>;
    async fn delete(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn update_last_seen(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;

    /// True si le user est marque comme parti (guild_members.left_at IS NOT NULL).
    /// False si actif OU si pas de ligne dans guild_members (jamais sync,
    /// par defaut on considere actif pour ne pas bloquer les anciens players).
    async fn is_left(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError>;

    /// Purge TOUTES les donnees de moderation d'un membre en une seule
    /// transaction atomique (voir `MEMBER_RESET_TABLES`). Renvoie, pour chaque
    /// table, la cle de reponse et le nombre de lignes supprimees. En cas
    /// d'erreur sur un DELETE, rollback complet (etat DB coherent).
    async fn reset_member(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<(&'static str, u64)>, DomainError>;

    /// Marque un membre comme parti (guild_members.left_at = NOW(), idempotent)
    /// et remet son wallet a 0. Renvoie le nombre de lignes guild_members MAJ.
    async fn mark_left(&self, guild_id: &str, user_id: &str) -> Result<u64, DomainError>;

    /// Marque un membre comme revenu (left_at = NULL, joined_at = NOW()).
    /// Renvoie le nombre de lignes guild_members MAJ.
    async fn mark_rejoined(&self, guild_id: &str, user_id: &str) -> Result<u64, DomainError>;

    /// Membres dont l'arrivee tombe dans les `days` prochains jours.
    ///
    /// Le filtrage se fait en SQL sur le jour et le mois : charger toute la
    /// guilde pour ne garder que trois anniversaires serait absurde sur un
    /// serveur de plusieurs centaines de membres.
    ///
    /// Exclut les bots et les partis : ni les uns ni les autres n'ont
    /// d'anniversaire a feter.
    async fn list_join_anniversaries(
        &self,
        guild_id: &str,
        days: i32,
    ) -> Result<Vec<JoinAnniversary>, DomainError>;

    /// Membres arrives dans les `days` derniers jours, les plus recents
    /// d'abord. Bots et partis exclus pour la meme raison.
    async fn list_recent_joins(
        &self,
        guild_id: &str,
        days: i32,
        limit: i64,
    ) -> Result<Vec<GuildMember>, DomainError>;
}
