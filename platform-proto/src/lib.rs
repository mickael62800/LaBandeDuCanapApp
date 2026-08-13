//! Contrats de transport internes de la plateforme.
//!
//! Les modules de premier niveau séparent les entités. Les packages Protobuf
//! restent inchangés afin de préserver les chemins gRPC et la compatibilité
//! binaire des messages.

pub mod atrium;
pub mod nexus;
pub mod sentinel;
