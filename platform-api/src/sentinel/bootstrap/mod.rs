//! Bootstrap : construction de l'etat applicatif (connexions infra + DI).
//!
//! Extrait de `main.rs` pour garder ce dernier concentre sur bind/serve.
//! Chaque phase de l'initialisation vit dans un sous-module dedie :
//! - `connections` : `connect_pg` / `connect_redis` (infra PostgreSQL + Redis).
//! - `inference` : `build_inference` / `build_broadcaster` (ONNX + pub/sub).
//! - `state` : le type `AppState` lui-meme (la composition root).
//! - `app_state` : `build_app_state` (assemble tous les repos/services).
//!
//! Les chemins publics restent stables (`crate::sentinel::bootstrap::ITEM`) via les
//! re-exports ci-dessous.
//!
//! # Pourquoi `AppState` vit ici
//!
//! Il residait dans `adapters/inbound/http/state.rs`, ce qui forcait
//! l'adaptateur gRPC a importer l'etat de l'adaptateur HTTP : deux adaptateurs
//! de meme niveau, l'un dependant de l'autre sans raison. L'etat applicatif
//! n'appartient a aucun protocole — il appartient a la composition root, que
//! chaque adaptateur consomme sans connaitre les autres.

mod app_state;
mod connections;
mod inference;
pub mod state;

pub use app_state::build_app_state;
pub use connections::{connect_pg, connect_redis};
pub use inference::{build_broadcaster, build_inference};
pub use state::AppState;
