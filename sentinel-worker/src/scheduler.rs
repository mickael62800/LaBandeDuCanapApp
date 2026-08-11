//! Scheduler central : enregistre tous les jobs periodiques avec leur
//! intervalle et delegue l'execution a `spawn_periodic` (impl commune
//! qui gere shutdown, panic catch, log lifecycle, metrics).
//!
//! Lecture de ce fichier = inventaire complet de ce que fait le worker.
//! Ajouter un job = ajouter une section ici + creer le module dans
//! `domains/{domain}/{job}.rs`.
//!
//! Note sur le `worker_name` passe a `spawn_periodic` et a
//! `is_worker_enabled` : on conserve les **noms d'origine par feature**
//! (cache-worker, audit-cache-worker, ...) plutot que de tout mettre
//! "sentinel-worker". Raison : les toggles `bot_guild_config` existants
//! sont indexes sur ces noms. Les changer obligerait a une migration DB
//! et casserait les configs guild deja en place.

use tracing::info;

use platform_common_worker::SupervisedTask;

use crate::config::CleanupConfig;
use crate::context::WorkerContext;
use crate::domains;

const WORKER_NAME: &str = "sentinel-worker";

mod maintenance;
mod moderation;
mod operations;
mod queues;

pub fn start(context: WorkerContext) -> Vec<SupervisedTask> {
    let mut tasks = Vec::new();
    maintenance::register(&context, &mut tasks);
    operations::register(&context, &mut tasks);
    queues::register(&context, &mut tasks);
    moderation::register(&context, &mut tasks);
    tasks
}
