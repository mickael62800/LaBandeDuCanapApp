//! Quarantaine de securite : un membre place en attente de verification
//! (captcha) avec un kick automatique si le delai expire (traite par le worker).

/// Une quarantaine encore active (non expiree), utilisee pour rehydrater le
/// tracker RAM du bot au demarrage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveQuarantine {
    pub guild_id: String,
    pub user_id: String,
}
