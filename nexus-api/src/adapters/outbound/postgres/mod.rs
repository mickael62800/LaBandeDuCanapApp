//! Adapters Postgres (sqlx) implementant les ports outbound de nexus-core.

pub mod casino;
pub mod coussin_bet_repository;
pub mod coussin_cooldown_repository;
pub mod coussin_insurance_repository;
pub mod coussin_inventory_repository;
pub mod coussin_prime_repository;
pub mod coussin_repository;
pub mod coussin_steal_repository;
pub mod game;
pub mod grand_salon_repository;
pub mod system;
pub mod wallet_repository;
pub mod wheel_repository;

use nexus_core::domain::errors::DomainError;

/// Convertit une erreur sqlx en erreur de domaine.
pub fn pg_err(e: sqlx::Error) -> DomainError {
    DomainError::Infrastructure(e.to_string())
}

/// Variante avec contexte (nom de table / repo) : le contexte apparait dans
/// le message d'erreur pour aider au debug (`"create game_server pg: ..."`).
pub(crate) fn pg_err_ctx(ctx: &'static str, e: sqlx::Error) -> DomainError {
    DomainError::Infrastructure(format!("{ctx} pg: {e}"))
}

/// Variante curryfiee de [`pg_err_ctx`] : capture le contexte et renvoie une
/// closure prete pour `.map_err(...)`.
pub(crate) fn pg_ctx(ctx: &'static str) -> impl FnOnce(sqlx::Error) -> DomainError {
    move |e| pg_err_ctx(ctx, e)
}
