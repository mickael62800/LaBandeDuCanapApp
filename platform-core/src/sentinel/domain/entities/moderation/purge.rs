//! Regles metier pour les endpoints de purge (infractions, audit logs,
//! system logs). Chaque type a une contrainte differente sur `days` :
//! - Infractions : `days >= 0` (0 signifie "tout supprimer").
//! - Audit logs / system logs : `days >= 1` (0 interdit, trop dangereux).

/// Valide la parametre `days` pour une purge d'infractions.
/// Regle : `days >= 0`. Une valeur 0 autorise la purge totale.
pub fn validate_purge_days_allow_zero(days: i32) -> Result<(), &'static str> {
    if days < 0 {
        return Err("days doit etre >= 0");
    }
    Ok(())
}

/// Valide la parametre `days` pour une purge qui refuse les "tout supprimer".
/// Regle : `days >= 1`.
pub fn validate_purge_days_strictly_positive(days: i32) -> Result<(), &'static str> {
    if days < 1 {
        return Err("days doit etre >= 1");
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/purge.rs"]
mod tests;
