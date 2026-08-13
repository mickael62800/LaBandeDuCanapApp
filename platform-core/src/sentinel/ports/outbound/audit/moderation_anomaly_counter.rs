use async_trait::async_trait;

/// Compteur d'evenements de moderation a fenetre glissante, cote serveur.
///
/// C'est l'etat mutable (le CALCUL) de la detection d'anomalie : il compte les
/// evenements recents par `(guild, categorie)` sur une fenetre glissante. La
/// DECISION (comparaison au seuil, reset) reste dans le service coeur.
#[async_trait]
pub trait ModerationAnomalyCounter: Send + Sync {
    /// Enregistre un evenement pour `(guild_id, category)` et retourne le
    /// nombre d'evenements presents dans la fenetre glissante de `window_secs`
    /// secondes (evenement courant inclus).
    async fn record(&self, guild_id: &str, category: &str, window_secs: u64) -> usize;

    /// Reinitialise le compteur pour `(guild_id, category)`. Appele apres une
    /// alerte pour eviter les alertes en boucle.
    async fn reset(&self, guild_id: &str, category: &str);
}
