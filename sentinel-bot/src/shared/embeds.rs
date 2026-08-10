//! Embeds normalisés du bot — ré-export du socle partagé.
//!
//! L'implémentation vit dans `platform-common-bot` : les trois bots doivent
//! rendre la même charte (couleurs de gravité, pied de page, cartes de
//! sanction). Ce module n'existe que pour garder le chemin d'import
//! historique `crate::shared::embeds::*` sur les ~20 fichiers qui
//! l'utilisent, sur le même modèle que `shared::discord_helpers`.
//!
//! Ce fichier a longtemps été une copie octet pour octet du socle : une
//! divergence ne se voyait pas au `cargo check`, seulement le jour où l'on
//! corrigeait un seul des deux exemplaires. Ne rien réimplémenter ici — un
//! embed propre à Sentinel se définit dans son module (cf.
//! `modules/voice/embeds.rs`).

pub use platform_common_bot::embeds::*;
