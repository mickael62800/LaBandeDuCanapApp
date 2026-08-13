//! Socle partage entre les plateformes Sentinel et Nexus.
//!
//! # Ce qui entre ici
//!
//! Du code **prouve identique** dans les deux stacks, et sans dependance a un
//! framework applicatif (ni axum, ni serenity). Aujourd'hui : le bus
//! d'evenements Redis Streams, qui existait en double a une constante pres.
//!
//! # Ce qui n'entre PAS ici
//!
//! Les clients HTTP des bots (`api_client.rs`) et les constructeurs d'embeds
//! divergent reellement entre les deux plateformes — 118 lignes communes sur
//! 517 pour le premier, 31 sur 199 pour le second. Les mutualiser reviendrait
//! a inventer une abstraction pour deux besoins differents, ce qui coute plus
//! cher que la duplication qu'elle supprime.
//!
//! Les middlewares HTTP vivent dans `platform-api::shared` : les y séparer
//! evite qu'un bot compile axum et tower-http pour rien.

pub mod config_flags;
pub mod errors;
pub mod event_bus;

pub use event_bus::default_consumer_name;
pub use event_bus::EventBus;
