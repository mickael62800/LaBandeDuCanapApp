//! GET/POST/DELETE /api/docker/* — administration Docker via le port `DockerHost`.
//!
//! Toutes les actions destructives (start/stop/restart/delete/prune) sont gardees
//! par require_superadmin. Les GET listing/inspect sont gates par moderator+ via
//! le middleware standard (suffisant : ils n'exposent que des metadonnees techniques).
//!
//! Le client Docker (bollard) vit dans l'adapter outbound `ops-adapters` ;
//! l'agregation « reclaimable » est une fonction pure du core (`compute_overview`).
//! Ce processus ne monte jamais le socket : il passe par le `docker-agent`.
//!
//! Decoupe par surface Docker : chaque sous-module porte ses handlers et leurs
//! DTO. Les helpers d'audit (`audited`, `record_docker_audit`, `actor_from`)
//! sont mutualises dans `audit`.

pub mod audit;
pub mod changes;
pub mod containers;
pub mod images;
pub mod networks;
pub mod overview;
pub mod prune;
pub mod volumes;
