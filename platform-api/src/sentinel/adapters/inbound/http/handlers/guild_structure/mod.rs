//! Construction de la structure d'un serveur (salons / categories) depuis le
//! panel web.
//!
//! Contrairement a `guild_backup`, qui delegue au bot via un event Redis, ces
//! actions passent par le port `DiscordApi` de l'API : l'utilisateur clique sur
//! « Valider » et attend le resultat: un fire-and-forget le laisserait devant
//! un ecran muet, a rafraichir jusqu'a deviner si ca a marche. Le meme choix a
//! ete fait pour la creation de roles (`community/discord_roles.rs`).

pub mod plan;
