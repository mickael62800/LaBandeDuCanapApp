//! Etat du domaine ops RESTE dans sentinel-api.
//!
//! L'essentiel de l'exploitation (Docker, securite de l'hote, regles
//! d'alerte, surveillance des conteneurs) vit desormais dans `ops-api`. Ne
//! subsistent ici que les ports encore consommes par Sentinel lui-meme :
//! sondes de sante, registre des services, logs techniques et compteur d'IP du
//! middleware. Ils partiront avec le groupe Logs.
//!
//! Conteneurs Docker, disques et sondes systeme, logs techniques des services,
//! securite de l'hote (certificat TLS, IP bannies, journal d'administration),
//! regles d'alerte.
//!
//! POURQUOI IL EST DISTINCT DE `SystemState`
//!
//! `system` melangeait deux choses de nature differente : le metier Discord de
//! la plateforme (tickets, OAuth, reset de guilde, lockdown, exports) et
//! l'exploitation de la machine qui l'heberge. Or cette machine heberge aussi
//! Nexus et Atrium : ces ecrans ne sont pas « du Sentinel », ils sont
//! transverses. La barre laterale du back-office les a deja separes en un
//! univers « Exploitation » ; ce sous-etat en est la contrepartie cote API.
//!
//! Un handler qui declare `State<OpsState>` se voit interdire par le
//! compilateur de toucher aux tickets ou a l'OAuth — et reciproquement.

use std::sync::Arc;

use axum::extract::FromRef;
use platform_core::ops::ports::outbound::service_registry::ServiceRegistry;
use platform_core::ops::ports::outbound::system_probe::SystemProbe;

use crate::sentinel::adapters::outbound::ws::broadcaster::EventBroadcaster;
use crate::sentinel::bootstrap::state::AppState;

/// Ports de l'exploitation de la machine hote.
#[derive(Clone)]
pub struct OpsState {
    // ── Etat de la machine et des services ──
    /// Sondes sante (taille/disponibilite BDD). Les handlers health/info
    /// passent par ici, jamais par `pg_pool`.
    pub system_probe: Arc<dyn SystemProbe>,
    // ── Journaux techniques ──
    /// Decouverte des bots et workers en ligne. Consomme par le tableau de
    /// bord, qui compose metier et sante des services au niveau du handler.
    pub service_registry: Arc<dyn ServiceRegistry>,

    // ── Securite de l'hote ──
    /// Suivi req/IP en memoire pour le ban automatique.
    pub rate_limiter:
        Option<Arc<crate::sentinel::adapters::outbound::system::rate_limiter::RateLimiter>>,

    // ── Dependances transverses ──
    pub broadcaster: Arc<EventBroadcaster>,
    pub redis_client: redis::Client,
}

impl FromRef<AppState> for OpsState {
    fn from_ref(state: &AppState) -> Self {
        state.ops.clone()
    }
}
