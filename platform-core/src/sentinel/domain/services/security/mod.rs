//! Détecteurs de sécurité — logique PURE (fenêtres glissantes anti-raid,
//! génération/suivi de captcha). Génériques sur la clé de serveur pour rester
//! sans dépendance Discord. Les actions Discord (lockdown, quarantine, envoi de
//! DM/boutons) restent dans l'adaptateur (sentinel-bot).

pub mod captcha;
pub mod raid_analyzer;
pub mod raid_detector;
