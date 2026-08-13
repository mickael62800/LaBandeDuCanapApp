//! Port inbound : use case Roue du Destin.

use async_trait::async_trait;

use crate::nexus::domain::entities::wheel::WheelSpin;
use crate::nexus::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct PlayWheelCommand {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct PlayWheelResult {
    pub spin: WheelSpin,
    pub balance_after: i64,
    /// True si la case est "memorable" (jackpot/licorne/bombe).
    pub is_memorable: bool,
}

#[async_trait]
pub trait PlayWheelUseCase: Send + Sync {
    /// 1 spin par joueur par jour (claim quotidien).
    /// Erreur `Validation` si le joueur a deja tire aujourd'hui.
    async fn spin(&self, cmd: PlayWheelCommand) -> Result<PlayWheelResult, DomainError>;

    /// Le joueur peut-il encore tirer aujourd'hui ?
    ///
    /// Permet a une interface de fermer son bouton AVANT le clic. La regle
    /// reste arbitree par `spin` : ce n'est qu'une indication.
    async fn can_spin(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError>;
}
