//! Domain Game Portal — entites pures de la plateforme de jeux.
//!
//! Aucune dependance infrastructure ici (pas de sqlx, pas de bollard,
//! pas de reqwest). Logique metier + types.

pub mod alert;
pub mod audit;
pub mod command;
pub mod config;
pub mod player_session;
pub mod presence;
pub mod quota;
pub mod schedule;
pub mod server;
pub mod session;
pub mod session_state;
pub mod template;
// Entités du portail de serveurs de jeu : templates, instances, sessions et
// événements de connexion. Le domaine ne dépend ni de Docker ni de Discord.
