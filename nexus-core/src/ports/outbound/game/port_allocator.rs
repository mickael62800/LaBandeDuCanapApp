//! Port allocator — alloue un port libre dans un range configurable.
//!
//! Implementation Redis-backed pour la coherence cross-process (l'API + le
//! worker peuvent allouer en parallele). Utilise SETNX sur une cle par
//! port + un set "allocated" pour le free / cleanup.

use async_trait::async_trait;

use crate::domain::errors::DomainError;

/// Type de port a allouer (range different).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortKind {
    Game,
    Rcon,
}

#[async_trait]
pub trait PortAllocator: Send + Sync {
    /// Reserve un port libre dans le range configure pour ce kind.
    /// Retourne le port alloue, ou Err si plus aucun libre.
    async fn allocate(
        &self,
        kind: PortKind,
        range_start: u16,
        range_end: u16,
        owner_key: &str,
    ) -> Result<u16, DomainError>;

    /// Reserve un bloc consecutif. Les implementations distribuees doivent
    /// rendre cette reservation atomique; le fallback sert aux adaptateurs de
    /// test et libere les ports deja pris si le bloc ne peut etre complete.
    async fn allocate_block(
        &self,
        kind: PortKind,
        range_start: u16,
        range_end: u16,
        width: u16,
        owner_key: &str,
    ) -> Result<u16, DomainError> {
        if width == 0 || range_end.saturating_sub(range_start) + 1 < width {
            return Err(DomainError::ValidationError(
                "plage insuffisante pour le bloc de ports".into(),
            ));
        }
        let last_start = range_end - (width - 1);
        for start in range_start..=last_start {
            let mut reserved = Vec::new();
            for port in start..start + width {
                if self.is_available(kind, port).await? {
                    match self.allocate(kind, port, port, owner_key).await {
                        Ok(_) => reserved.push(port),
                        Err(_) => break,
                    }
                } else {
                    break;
                }
            }
            if reserved.len() == width as usize {
                return Ok(start);
            }
            for port in reserved {
                let _ = self.release(kind, port).await;
            }
        }
        Err(DomainError::ValidationError(
            "aucun bloc de ports libre dans le range configure".into(),
        ))
    }

    /// Libere un port (a appeler au stop / delete).
    async fn release(&self, kind: PortKind, port: u16) -> Result<(), DomainError>;

    /// Verifie qu'un port est dispo (sans le reserver). Pour le reconciler.
    async fn is_available(&self, kind: PortKind, port: u16) -> Result<bool, DomainError>;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Mutex;

    use super::*;

    struct MemoryAllocator(Mutex<BTreeSet<u16>>);

    #[async_trait]
    impl PortAllocator for MemoryAllocator {
        async fn allocate(
            &self,
            _: PortKind,
            range_start: u16,
            range_end: u16,
            _: &str,
        ) -> Result<u16, DomainError> {
            let mut ports = self.0.lock().unwrap();
            for port in range_start..=range_end {
                if ports.insert(port) {
                    return Ok(port);
                }
            }
            Err(DomainError::ValidationError("port occupe".into()))
        }

        async fn release(&self, _: PortKind, port: u16) -> Result<(), DomainError> {
            self.0.lock().unwrap().remove(&port);
            Ok(())
        }

        async fn is_available(&self, _: PortKind, port: u16) -> Result<bool, DomainError> {
            Ok(!self.0.lock().unwrap().contains(&port))
        }
    }

    #[tokio::test]
    async fn block_allocation_skips_ports_adjacent_to_an_existing_valheim_block() {
        // 25501 appartient deja a un autre serveur : 25500 ne peut donc pas
        // etre le debut d'un bloc Valheim (25500, 25501, 25502).
        let allocator = MemoryAllocator(Mutex::new(BTreeSet::from([25501])));
        let start = allocator
            .allocate_block(PortKind::Game, 25500, 25508, 3, "valheim-b")
            .await
            .unwrap();
        assert_eq!(start, 25502);
        let ports = allocator.0.lock().unwrap();
        assert!(ports.contains(&25502));
        assert!(ports.contains(&25503));
        assert!(ports.contains(&25504));
    }

    #[tokio::test]
    async fn block_allocation_rejects_a_range_smaller_than_valheim_needs() {
        let allocator = MemoryAllocator(Mutex::new(BTreeSet::new()));
        assert!(allocator
            .allocate_block(PortKind::Game, 25500, 25501, 3, "valheim")
            .await
            .is_err());
    }
}
