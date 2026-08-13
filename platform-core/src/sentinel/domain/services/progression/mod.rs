//! Progression — suivi in-memory de l'activité (messages + temps vocal actif)
//! avec anti-farm XP AFK. Logique PURE (clés (guild_id, user_id) en u64, aucun
//! type Discord). Partageable par le bot et tout autre adaptateur.

pub mod nickname;
pub mod role_tiers;
pub mod tracker;
