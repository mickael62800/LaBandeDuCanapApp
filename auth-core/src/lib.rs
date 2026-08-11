//! Cœur hexagonal de l'identité : OAuth2 Discord, sessions web, gate superadmin.
//!
//! # Pourquoi une plateforme à part
//!
//! L'identité vivait dans `sentinel-api`. Conséquence : Nexus et Atrium
//! n'avaient aucun moyen de savoir qui appelle, et la passerelle nginx devait
//! demander son avis à `sentinel-api` (`auth_request → /api/auth/nexus-access`)
//! avant de relayer vers eux. Sentinel était donc une dépendance d'exécution de
//! toutes les autres plateformes — celle qui, si elle tombe, ferme le
//! back-office entier.
//!
//! Le même geste avait déjà été fait pour l'exploitation (`ops-core` /
//! `ops-api`) : l'identité n'appartient pas plus à Sentinel que les sondes de
//! la machine hôte. Ici, `sentinel-api` redevient un consommateur comme les
//! deux autres.
//!
//! # Ce que ce crate contient, et pas
//!
//! Il décide **qui vous êtes** et **si vous avez le droit d'entrer dans le
//! back-office**. Il ne sait rien des guildes Discord, des rôles applicatifs ni
//! des composants : depuis le passage du back-office en superadmin-only, la
//! règle d'autorisation tient en une ligne (« ce compte figure-t-il dans la
//! liste ? »), ce qui est précisément ce qui rend l'extraction possible.
//!
//! Aucune dépendance infra : pas de sqlx, pas d'axum, pas de reqwest, pas de
//! redis. Tout passe par les ports de `ports::outbound`.

pub mod application;
pub mod domain;
pub mod ports;
