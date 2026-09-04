//! Immutable application generations with atomic last-known-good publishing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::{GenerationId, Request, WorkerError};

#[derive(Debug, Clone)]
pub struct Generation {
    pub id: GenerationId,
    pub bundle: Arc<str>,
    pub filename: Arc<str>,
}

pub struct GenerationRegistry {
    next_id: AtomicU64,
    current: RwLock<Option<Arc<Generation>>>,
}

impl Default for GenerationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GenerationRegistry {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            current: RwLock::new(None),
        }
    }

    /// Validate a candidate before publishing it. The old generation remains
    /// current whenever validation fails.
    pub fn publish(
        &self,
        bundle: impl Into<Arc<str>>,
        filename: impl Into<Arc<str>>,
    ) -> Result<Arc<Generation>, WorkerError> {
        let bundle = bundle.into();
        let filename = filename.into();
        if bundle.trim().is_empty() {
            return Err(WorkerError::Execution(
                "generation bundle must not be empty".into(),
            ));
        }
        if filename.trim().is_empty() {
            return Err(WorkerError::Execution(
                "generation filename must not be empty".into(),
            ));
        }
        let generation = Arc::new(Generation {
            id: GenerationId(self.next_id.fetch_add(1, Ordering::Relaxed)),
            bundle,
            filename,
        });
        *self
            .current
            .write()
            .map_err(|_| WorkerError::Execution("generation lock poisoned".into()))? =
            Some(Arc::clone(&generation));
        Ok(generation)
    }

    pub fn current(&self) -> Option<Arc<Generation>> {
        self.current.read().ok().and_then(|current| current.clone())
    }

    /// Pin the currently published generation into a request. Holding the
    /// returned Arc lets callers keep the generation alive until completion.
    pub fn pin(&self, request: &mut Request) -> Option<Arc<Generation>> {
        let generation = self.current()?;
        request.generation = Some(generation.id);
        Some(generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_reload_keeps_last_known_good_generation() {
        let registry = GenerationRegistry::new();
        let first = registry.publish("bundle-1", "one.js").unwrap();
        assert!(registry.publish("", "broken.js").is_err());
        assert_eq!(registry.current().unwrap().id, first.id);
    }

    #[test]
    fn requests_pin_an_immutable_generation_id() {
        let registry = GenerationRegistry::new();
        let generation = registry.publish("bundle-1", "one.js").unwrap();
        let mut request = Request {
            version: crate::PROTOCOL_VERSION,
            request_id: "r-1".into(),
            generation: None,
            method: "GET".into(),
            target: "/".into(),
            http_version: "HTTP/1.1".into(),
            headers: Vec::new(),
            body: crate::BodyEnvelope::Empty,
        };
        let pinned = registry.pin(&mut request).unwrap();
        assert_eq!(pinned.id, generation.id);
        assert_eq!(request.generation, Some(generation.id));
    }
}
