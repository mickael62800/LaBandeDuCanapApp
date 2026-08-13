//! Domaine de SAUVEGARDE / RESTAURATION de serveur Discord (`guild_backup`).
//!
//! A NE PAS confondre avec le domaine `audit::snapshots` (analytics : compte
//! d'activite quotidien). Ici on capture la STRUCTURE complete d'un serveur
//! (roles, categories, salons, permissions/overwrites, reglages, bans, emojis,
//! mapping membre -> roles) sous forme d'un `GuildSnapshot` serialisable, afin
//! de pouvoir la stocker versionnee puis la RESTAURER sur un serveur neuf.
//!
//! Ces types sont le CONTRAT partage bot <-> api (via serde/JSON). Le bot
//! (phase 2) produira un `GuildSnapshot` a la capture et le re-lira a la
//! restauration ; l'API ne fait que le stocker/lire en JSONB.
//!
//! # `old_id`
//!
//! Tous les champs `*_old_id` contiennent l'ID Discord d'ORIGINE (celui du
//! serveur capture). A la restauration, le bot cree les nouvelles ressources
//! et construit une table de correspondance `old_id -> new_id` pour recabler
//! les references (parent de salon, cible d'overwrite, roles d'un membre...).

pub mod pending_role_grant;
pub mod snapshot;

pub use pending_role_grant::*;
pub use snapshot::*;
