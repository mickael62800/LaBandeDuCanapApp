//! Delais entre deux actions d'un meme joueur.
//!
//! Un seul port pour toutes les actions plutot qu'un compteur par service :
//! la table est la meme, et trois implementations auraient diverge des la
//! premiere correction.
//!
//! L'action est une chaine libre (`combat`, `bet`, `prime`, `class`) : elle
//! sert de cle et n'est jamais montree telle quelle au joueur.

use async_trait::async_trait;

use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait CoussinCooldownRepository: Send + Sync {
    /// Secondes restantes avant que l'action redevienne possible, ou `None`
    /// si elle l'est deja.
    ///
    /// On rend le RESTANT et non un simple booleen : c'est ce qui permet de
    /// dire « reessaie dans 4 min » plutot que d'annoncer une duree
    /// theorique, qui serait fausse partout sauf a la premiere seconde.
    async fn remaining_seconds(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<i64>, DomainError>;

    /// Arme le delai apres une action reussie. `minutes` a 0 n'ecrit rien :
    /// un serveur sans delai ne doit pas accumuler des lignes inutiles.
    async fn arm(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        minutes: i64,
    ) -> Result<(), DomainError>;
}
