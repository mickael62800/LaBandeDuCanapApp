//! Identité résolue et règle d'accès au back-office.

use serde::{Deserialize, Serialize};

/// Identité Discord d'un appelant web, telle que `GET /users/@me` la renvoie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
}

/// Couple de jetons rendu par Discord (code d'autorisation ou refresh).
#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    /// Durée de vie de l'access token, en secondes, telle que Discord l'annonce.
    pub expires_in_secs: i64,
}

/// Verdict rendu à un appelant : qui, et a-t-il le droit d'entrer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessVerdict {
    pub discord_user_id: String,
    pub granted: bool,
}

/// Règle d'accès au back-office : la liste des comptes autorisés.
///
/// # Fail-closed
///
/// Liste vide = PERSONNE ne passe. C'est volontaire, et c'est la propriété la
/// plus importante du crate : mieux vaut un back-office inaccessible qu'un
/// back-office ouvert. Une erreur de configuration doit fermer la porte, pas
/// l'ouvrir.
#[derive(Debug, Clone, Default)]
pub struct SuperadminPolicy {
    allowed: Vec<String>,
}

impl SuperadminPolicy {
    pub fn new(allowed: Vec<String>) -> Self {
        Self {
            // Un identifiant vide dans la liste (virgule en trop dans le .env)
            // ne doit pas devenir un joker : on les écarte à la construction.
            allowed: allowed
                .into_iter()
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty())
                .collect(),
        }
    }

    /// Découpe une liste CSV telle qu'elle arrive de l'environnement.
    pub fn from_csv(raw: &str) -> Self {
        Self::new(raw.split(',').map(|s| s.to_string()).collect())
    }

    pub fn grants(&self, discord_user_id: &str) -> bool {
        self.allowed.iter().any(|id| id == discord_user_id)
    }

    pub fn is_empty(&self) -> bool {
        self.allowed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liste_vide_ne_laisse_passer_personne() {
        let policy = SuperadminPolicy::new(vec![]);
        assert!(policy.is_empty());
        assert!(!policy.grants("123"));
        assert!(!policy.grants(""));
    }

    /// Une virgule en trop dans `SUPERADMIN_USER_IDS` produisait une entrée
    /// vide. Sans ce filtre, un appelant sans identité résolue (`""`) aurait
    /// été autorisé.
    #[test]
    fn les_entrees_vides_ne_sont_pas_un_joker() {
        let policy = SuperadminPolicy::from_csv("123,,456,");
        assert!(policy.grants("123"));
        assert!(policy.grants("456"));
        assert!(!policy.grants(""));
    }

    #[test]
    fn les_espaces_autour_des_identifiants_sont_tolores() {
        let policy = SuperadminPolicy::from_csv(" 123 , 456 ");
        assert!(policy.grants("123"));
        assert!(policy.grants("456"));
    }
}
