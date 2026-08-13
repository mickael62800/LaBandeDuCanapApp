//! Implémentation Postgres du Unit of Work.
//!
//! `PgTx` enveloppe une `sqlx::Transaction<'static, Postgres>`. Le helper
//! `as_pg(tx)` permet aux impls de repo Postgres de récupérer leur tx
//! concrète depuis un `&mut dyn DbTx` opaque.

use std::any::Any;

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;

use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::uow::{DbTx, UnitOfWork};

use super::pg_err;

pub struct PgTx(pub Transaction<'static, Postgres>);

impl DbTx for PgTx {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct PgUnitOfWork {
    pool: PgPool,
}

impl PgUnitOfWork {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UnitOfWork for PgUnitOfWork {
    async fn begin(&self) -> Result<Box<dyn DbTx>, DomainError> {
        let tx = self.pool.begin().await.map_err(pg_err)?;
        Ok(Box::new(PgTx(tx)))
    }

    async fn commit(&self, tx: Box<dyn DbTx>) -> Result<(), DomainError> {
        let pg = downcast_owned(tx)?;
        pg.0.commit().await.map_err(pg_err)
    }

    async fn rollback(&self, tx: Box<dyn DbTx>) -> Result<(), DomainError> {
        let pg = downcast_owned(tx)?;
        pg.0.rollback().await.map_err(pg_err)
    }
}

fn downcast_owned(tx: Box<dyn DbTx>) -> Result<Box<PgTx>, DomainError> {
    let any: Box<dyn Any> = unsafe {
        // SAFETY: DbTx: Any + Send. On reconstruit un Box<dyn Any> à partir
        // du même pointeur. Pas d'alias, pas de double free.
        let raw = Box::into_raw(tx);
        Box::from_raw(raw as *mut dyn Any)
    };
    any.downcast::<PgTx>()
        .map_err(|_| DomainError::Internal("UnitOfWork: tx must be PgTx".into()))
}

/// Helper pour les impls de repo Postgres : extrait la `&mut Transaction`
/// concrète depuis un `&mut dyn DbTx`. Panique si l'impl injecte un autre
/// backend — cohérent avec une architecture mono-Postgres.
pub fn as_pg(tx: &mut dyn DbTx) -> &mut Transaction<'static, Postgres> {
    &mut tx
        .as_any_mut()
        .downcast_mut::<PgTx>()
        .expect("DbTx must be PgTx in production code")
        .0
}
