//! Application-to-worker-group ownership.
//!
//! An application gets a distinct WorkerGroup. Registering two applications
//! therefore cannot accidentally make them share a Bun VM or a worker process.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use super::pool::WorkerPool;

#[derive(Clone)]
pub struct WorkerGroup {
    pool: Arc<WorkerPool>,
}

impl WorkerGroup {
    pub fn new(pool: Arc<WorkerPool>) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &Arc<WorkerPool> {
        &self.pool
    }
}

pub struct Application {
    name: String,
    workers: WorkerGroup,
}

impl Application {
    pub fn new(name: impl Into<String>, workers: WorkerGroup) -> Self {
        Self {
            name: name.into(),
            workers,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn workers(&self) -> &WorkerGroup {
        &self.workers
    }
}

#[derive(Default)]
pub struct ApplicationRegistry {
    applications: RwLock<HashMap<String, Arc<Application>>>,
}

impl ApplicationRegistry {
    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<ApplicationRegistry> = OnceLock::new();
        REGISTRY.get_or_init(Self::default)
    }

    pub fn register(&self, application: Application) -> Arc<Application> {
        let application = Arc::new(application);
        self.applications
            .write()
            .expect("application registry lock poisoned")
            .insert(application.name.clone(), Arc::clone(&application));
        application
    }

    pub fn get(&self, name: &str) -> Option<Arc<Application>> {
        self.applications
            .read()
            .expect("application registry lock poisoned")
            .get(name)
            .cloned()
    }

    pub fn remove(&self, name: &str) -> Option<Arc<Application>> {
        self.applications
            .write()
            .expect("application registry lock poisoned")
            .remove(name)
    }

    pub fn len(&self) -> usize {
        self.applications
            .read()
            .expect("application registry lock poisoned")
            .len()
    }
}
