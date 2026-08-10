//! Regles metier de l'EXPLOITATION : la machine hote, pas Discord.
//!
//! Sondes systeme, conteneurs Docker, logs techniques des services, securite
//! de l'hote (TLS, IP bannies, journal d'administration), regles d'alerte.
//!
//! # Pourquoi un crate a part
//!
//! Cette machine heberge Sentinel, Nexus ET Atrium : ses ecrans ne sont « du
//! Sentinel » que par accident d'histoire. Le domaine vivait dans
//! `sentinel-core::*::ops`, melange au metier Discord (tickets, OAuth, reset
//! de guilde) sous le nom trompeur de `system`.
//!
//! # Sens des dependances
//!
//! `ops-core` ne depend d'aucune plateforme. C'est `sentinel-core` qui
//! depend de lui, pour un seul port : `ServiceRegistry`, que son tableau de
//! bord d'audit utilise pour compter les bots et workers en ligne. Rendre
//! compte de la sante des services est bien un usage de l'exploitation par le
//! metier, pas l'inverse.

pub mod application;
pub mod domain;
pub mod ports;
