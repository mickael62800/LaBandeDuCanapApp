//! Unit of Work — abstraction de transaction pour les ports.
//!
//! `DbTx` est un handle opaque sur une transaction en cours. Les adapters
//! (Postgres, mocks) downcastent vers leur type concret via `as_any_mut()`.
//! `UnitOfWork` ouvre/commit/rollback ces transactions.
//!
//! Le contexte Sentinel ne dépend ainsi d'aucune bibliothèque d'infrastructure.

use std::any::Any;

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;

/// Handle opaque sur une transaction. Les adapters downcastent via
/// `as_any_mut()` vers leur type concret (`PgTx`, `NoopTx`, ...).
pub trait DbTx: Any + Send {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[async_trait]
pub trait UnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn DbTx>, DomainError>;
    async fn commit(&self, tx: Box<dyn DbTx>) -> Result<(), DomainError>;
    async fn rollback(&self, tx: Box<dyn DbTx>) -> Result<(), DomainError>;
}
