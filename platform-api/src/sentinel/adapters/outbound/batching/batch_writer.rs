//! BatchWriter<T> — buffer in-memory + flush periodique pour inserts batch.
//!
//! # Architecture
//!
//! - `mpsc::channel` bornee (drop si plein → pas de blocage du request path)
//! - Flusher task spawn une fois au demarrage, consume la Receiver
//! - Deux triggers de flush : taille batch atteinte OR interval tick
//! - Sur drop du dernier Sender (shutdown API), le flusher draine le reste et exit
//!
//! # Configuration typique
//!
//! ```ignore
//! BatchWriterConfig {
//!     flush_interval: Duration::from_millis(500),
//!     max_batch_size: 100,
//!     channel_capacity: 10_000,
//! }
//! ```

use std::future::Future;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tracing::debug;
use tracing::warn;
#[derive(Debug, Clone, Copy)]
pub struct BatchWriterConfig {
    /// Intervalle max entre deux flushs (meme si le batch n'est pas plein).
    pub flush_interval: Duration,
    /// Taille max d'un batch avant flush immediat.
    pub max_batch_size: usize,
    /// Capacite du channel mpsc. Si plein, les envois sont drop.
    pub channel_capacity: usize,
}

impl Default for BatchWriterConfig {
    fn default() -> Self {
        Self {
            flush_interval: Duration::from_millis(500),
            max_batch_size: 100,
            channel_capacity: 10_000,
        }
    }
}

/// Handle cote producteur : wrap un `mpsc::Sender` avec la politique "drop si plein".
#[derive(Clone)]
pub struct BatchWriter<T: Send + 'static> {
    tx: mpsc::Sender<T>,
    label: &'static str,
}

impl<T: Send + 'static> BatchWriter<T> {
    /// Cree un BatchWriter + spawn la flusher task en background.
    ///
    /// `label` est utilise dans les logs (ex: "logs", "audit_logs").
    /// `flush_fn` recoit un `Vec<T>` non-vide a chaque flush, doit retourner
    /// `Ok(())` ou une erreur (qui sera logguee, sans retry — les entries sont perdues).
    pub fn spawn<F, Fut>(label: &'static str, config: BatchWriterConfig, flush_fn: F) -> Self
    where
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<T>(config.channel_capacity);

        tokio::spawn(run_flusher(label, rx, config, flush_fn));

        Self { tx, label }
    }

    /// Enqueue un item non-bloquant. Retourne `true` si ajoute, `false` si drop (queue pleine).
    pub fn try_send(&self, item: T) -> bool {
        match self.tx.try_send(item) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!(label = %self.label, "BatchWriter queue pleine — entry dropped");
                false
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                warn!(label = %self.label, "BatchWriter channel ferme — entry dropped");
                false
            }
        }
    }
}

async fn run_flusher<T, F, Fut>(
    label: &'static str,
    mut rx: mpsc::Receiver<T>,
    config: BatchWriterConfig,
    flush_fn: F,
) where
    T: Send + 'static,
    F: Fn(Vec<T>) -> Fut + Send + Sync,
    Fut: Future<Output = Result<(), String>> + Send,
{
    let mut buffer: Vec<T> = Vec::with_capacity(config.max_batch_size);
    let mut interval = tokio::time::interval(config.flush_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    // Premier tick immediat — on le consomme pour ne pas flush un batch vide au start.
    interval.tick().await;

    debug!(label = %label, "BatchWriter flusher demarre");

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if !buffer.is_empty() {
                    let batch = std::mem::replace(&mut buffer, Vec::with_capacity(config.max_batch_size));
                    do_flush(label, &flush_fn, batch).await;
                }
            }
            maybe_item = rx.recv() => {
                match maybe_item {
                    Some(item) => {
                        buffer.push(item);
                        // Drainer sans attendre pour remplir le batch d'un coup
                        while buffer.len() < config.max_batch_size {
                            match rx.try_recv() {
                                Ok(next) => buffer.push(next),
                                Err(_) => break,
                            }
                        }
                        if buffer.len() >= config.max_batch_size {
                            let batch = std::mem::replace(&mut buffer, Vec::with_capacity(config.max_batch_size));
                            do_flush(label, &flush_fn, batch).await;
                        }
                    }
                    None => {
                        // Channel ferme → flush final et exit
                        if !buffer.is_empty() {
                            let batch = std::mem::take(&mut buffer);
                            do_flush(label, &flush_fn, batch).await;
                        }
                        debug!(label = %label, "BatchWriter flusher arrete (channel closed)");
                        return;
                    }
                }
            }
        }
    }
}

async fn do_flush<T, F, Fut>(label: &'static str, flush_fn: &F, batch: Vec<T>)
where
    T: Send + 'static,
    F: Fn(Vec<T>) -> Fut,
    Fut: Future<Output = Result<(), String>> + Send,
{
    let count = batch.len();
    let start = std::time::Instant::now();
    match flush_fn(batch).await {
        Ok(()) => {
            debug!(
                label = %label,
                count,
                elapsed_ms = start.elapsed().as_millis() as u64,
                "Batch flushed"
            );
        }
        Err(e) => {
            warn!(label = %label, count, error = %e, "Batch flush failed (entries perdues)");
        }
    }
}

#[cfg(test)]
#[path = "tests/batch_writer.rs"]
mod tests;
