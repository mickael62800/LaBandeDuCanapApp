use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::poll::{Poll, UpsertPollCommand};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait PollRepository: Send + Sync {
    /// Sondages d'une guilde, options et decompte des voix inclus.
    ///
    /// `open_only` sert la page publique, qui ne montre que ce sur quoi on
    /// peut encore voter.
    async fn list(
        &self,
        guild_id: &str,
        open_only: bool,
        limit: i64,
    ) -> Result<Vec<Poll>, DomainError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Poll>, DomainError>;

    /// Cree le sondage ET ses options : un sondage sans choix n'existe pas,
    /// les deux ecritures doivent etre atomiques.
    async fn create(&self, cmd: &UpsertPollCommand) -> Result<Poll, DomainError>;

    async fn set_closed(&self, id: Uuid, closed: bool) -> Result<bool, DomainError>;

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError>;

    /// Enregistre un vote. Changer d'avis remplace le precedent — d'ou
    /// l'UPSERT plutot qu'un INSERT qui echouerait.
    ///
    /// Renvoie `false` si l'option n'appartient pas au sondage : sans ce
    /// controle, un client pourrait voter pour l'option d'un autre sondage.
    async fn cast_vote(
        &self,
        poll_id: Uuid,
        option_id: Uuid,
        user_id: &str,
    ) -> Result<bool, DomainError>;

    /// Option choisie par un membre, pour pre-cocher son vote a l'affichage.
    async fn vote_of(&self, poll_id: Uuid, user_id: &str) -> Result<Option<Uuid>, DomainError>;
}
