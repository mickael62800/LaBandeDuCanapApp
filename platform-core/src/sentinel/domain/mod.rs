//! Regles et modeles metier de Sentinel.
//!
//! Les contextes moderation, audit, communaute, sauvegarde, IA et systeme
//! restent independants de l'infrastructure. Les calculs purs et transitions
//! sont testes ici ; SQL, HTTP et Discord appartiennent aux adaptateurs.

pub mod entities;
pub mod enums;
pub mod errors;
pub mod services;
