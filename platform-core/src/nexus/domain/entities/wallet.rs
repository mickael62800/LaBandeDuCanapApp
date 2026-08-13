//! Wallet partage Nexus — coins par (guild, user). Socle commun de tous les
//! jeux de la plateforme.
//!
//! # Garde anti-negatif (regle systemique)
//!
//! Un solde ne passe JAMAIS en negatif (CHECK `coins >= 0` en DB en dernier
//! filet). Deux politiques selon l'origine du debit :
//!
//! - Les JEUX debitent avec CLAMP (`debit_clamped`) : si le solde est
//!   insuffisant, on debite ce qui reste (comportement historique "best
//!   effort" repris de `clamp_debit_to_balance` du wallet Sentinel, conserve
//!   pour la wheel).
//! - Les TRANSFERTS entre joueurs REFUSENT explicitement (`debit_exact`) :
//!   pas de clamp, erreur de validation si le solde est insuffisant.
//!
//! # Sources de transaction
//!
//! Chaque mutation est journalisee avec une `source` obligatoire (String
//! validee, enum ouverte) : `"wheel_payout"`, `"wheel_loss"`,
//! `"transfer_in"`, `"transfer_out"`, `"starting_coins"`, `"admin_grant"`,
//! et les futurs jeux. Le montant est signe (positif = credit, negatif =
//! debit), la raison est optionnelle.

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use crate::nexus::domain::errors::DomainError;

/// Solde de depart par defaut d'un wallet (coins offerts a la creation).
/// Valeur historique du wallet Sentinel (`DEFAULT_STARTING_COINS`, migration
/// `285_economy_starting_coins_config.sql` : default "100").
pub const DEFAULT_STARTING_COINS: i64 = 100;

/// Plafond metier d'une operation wallet unitaire (borne historique
/// `MAX_WALLET_AMOUNT` du wallet Sentinel). Empeche un montant absurde qui
/// saturerait la ligne ou dupliquerait des coins en masse.
pub const MAX_WALLET_AMOUNT: i64 = 1_000_000_000_000; // 1e12

/// Resout le solde de depart d'un wallet : override par-guild si configure
/// (jamais negatif), sinon le defaut historique. Regle metier pure.
pub fn resolve_starting_coins(guild_override: Option<i64>) -> i64 {
    guild_override.unwrap_or(DEFAULT_STARTING_COINS).max(0)
}

/// Valide une `source` de transaction : non vide, <= 40 caracteres (taille
/// de la colonne `nexus_wallet_transactions.source`).
pub fn validate_source(source: &str) -> Result<(), DomainError> {
    if source.trim().is_empty() {
        return Err(DomainError::Validation(
            "source de transaction obligatoire".into(),
        ));
    }
    if source.len() > 40 {
        return Err(DomainError::Validation(
            "source de transaction trop longue (max 40)".into(),
        ));
    }
    Ok(())
}

/// Regles PURES d'un transfert entre joueurs (`/donner` historique) :
/// - pas d'auto-transfert ;
/// - montant strictement positif et borne par `MAX_WALLET_AMOUNT` ;
/// - solde emetteur suffisant — REFUS explicite, jamais de clamp.
pub fn validate_transfer(
    from_user: &str,
    to_user: &str,
    amount: i64,
    from_balance: i64,
) -> Result<(), DomainError> {
    if from_user == to_user {
        return Err(DomainError::Validation(
            "Impossible de transferer vers soi-meme".into(),
        ));
    }
    if amount <= 0 {
        return Err(DomainError::Validation(
            "Le montant doit etre positif".into(),
        ));
    }
    if amount > MAX_WALLET_AMOUNT {
        return Err(DomainError::Validation("Montant trop eleve".into()));
    }
    if from_balance < amount {
        return Err(DomainError::Validation(format!(
            "Solde insuffisant ({from_balance} coins)"
        )));
    }
    Ok(())
}

/// Clamp une limite de pagination demandee dans [1, max], avec un defaut si
/// absente. Regle pure partagee entre historique et leaderboard.
pub fn clamp_limit(requested: Option<i64>, default: i64, max: i64) -> i64 {
    requested.unwrap_or(default).clamp(1, max)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wallet {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
}

impl Wallet {
    /// Wallet vierge (nouveau joueur) : 0 coins. Le solde de depart est
    /// credite par le service (`get_or_create`) via `resolve_starting_coins`.
    pub fn new(guild_id: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self {
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            username: String::new(),
            coins: 0,
            total_earned: 0,
            total_spent: 0,
        }
    }

    /// Credite `amount` coins (strictement positif).
    pub fn credit(&mut self, amount: i64) -> Result<(), DomainError> {
        if amount <= 0 {
            return Err(DomainError::Validation(
                "montant de credit invalide (doit etre > 0)".into(),
            ));
        }
        self.coins = self.coins.saturating_add(amount);
        self.total_earned = self.total_earned.saturating_add(amount);
        Ok(())
    }

    /// Debite `amount` coins (strictement positif), clampe au solde.
    /// Retourne le montant REELLEMENT debite (peut etre < amount, jamais < 0).
    /// Politique JEUX : best-effort, jamais d'echec pour solde insuffisant.
    pub fn debit_clamped(&mut self, amount: i64) -> Result<i64, DomainError> {
        if amount <= 0 {
            return Err(DomainError::Validation(
                "montant de debit invalide (doit etre > 0)".into(),
            ));
        }
        let actual = amount.min(self.coins);
        self.coins -= actual;
        self.total_spent = self.total_spent.saturating_add(actual);
        Ok(actual)
    }

    /// Debite `amount` coins (strictement positif) SANS clamp : erreur de
    /// validation si le solde est insuffisant. Politique TRANSFERTS.
    pub fn debit_exact(&mut self, amount: i64) -> Result<(), DomainError> {
        if amount <= 0 {
            return Err(DomainError::Validation(
                "montant de debit invalide (doit etre > 0)".into(),
            ));
        }
        if self.coins < amount {
            return Err(DomainError::Validation(format!(
                "Solde insuffisant ({} coins)",
                self.coins
            )));
        }
        self.coins -= amount;
        self.total_spent = self.total_spent.saturating_add(amount);
        Ok(())
    }
}

/// Mutation appliquee au wallet, a journaliser dans
/// `nexus_wallet_transactions` (positif = credit, negatif = debit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletMutation {
    pub amount: i64,
    pub balance_after: i64,
    pub source: String,
    pub description: String,
    /// Raison optionnelle (libre, saisie joueur/admin).
    pub reason: Option<String>,
}

/// Ligne d'historique lue depuis `nexus_wallet_transactions`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub amount: i64,
    pub balance_after: i64,
    pub source: String,
    pub description: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "tests/wallet.rs"]
mod tests;
