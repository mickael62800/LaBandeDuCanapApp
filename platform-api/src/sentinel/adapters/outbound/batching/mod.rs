//! Phase 5C — Batch writes pour les tables event-heavy.
//!
//! Les endpoints qui emettent beaucoup de logs (middleware API, send_log depuis
//! les bots, audit events) passent par un `BatchWriter<T>` qui bufferise les
//! entries en memoire et les flush periodiquement via un INSERT multi-rows.
//!
//! **Sémantique at-most-once** : en cas de crash de l'API, le buffer en memoire
//! est perdu. Ce trade-off est acceptable pour des logs (non-transactionnel) en
//! echange d'un throughput 10-50x superieur et d'une charge DB reduite.
//!
//! Pour les ecritures critiques (infractions, transactions economiques), continuer
//! a utiliser les repositories synchrones non-batches.

pub mod audit_log_batcher;
pub mod batch_writer;
pub mod log_batcher;
