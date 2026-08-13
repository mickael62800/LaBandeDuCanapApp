//! Re-export de `AppState` pour les chemins historiques.
//!
//! Le type vit desormais dans `crate::sentinel::bootstrap::state` : c'est la composition
//! root, pas un detail de l'adaptateur HTTP. Le laisser ici obligeait
//! l'adaptateur gRPC a faire
//! `use crate::sentinel::adapters::inbound::http::state::AppState`, c'est-a-dire a
//! dependre d'un adaptateur frere.
//!
//! Ce module ne survit que pour eviter de reecrire ~380 imports d'un coup.
//! Les nouveaux fichiers doivent importer `crate::sentinel::bootstrap::AppState`.

pub use crate::sentinel::bootstrap::state::AppState;
