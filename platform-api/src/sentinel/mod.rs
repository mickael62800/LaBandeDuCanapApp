//! `sentinel-api` — adaptateurs et composition root de l'API Sentinel.
//!
//! Le métier vit dans `platform_core::sentinel` : `domain`, `application` et `ports` se
//! referencent directement par `platform_core::sentinel::…`. Ce crate re-exportait
//! auparavant `platform_core::sentinel::{ports, application}` sous `crate::sentinel::`, ce qui
//! donnait deux chemins valides pour le meme type — au point qu'un meme
//! fichier melangeait les deux formes. Les re-exports ont ete retires.

pub mod adapters;
pub mod bootstrap;
pub mod config;
pub mod jobs;
