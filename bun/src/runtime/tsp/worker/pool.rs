//! Least-active WorkerPool for one application.
//!
//! Every slot owns a separate WorkerManager and therefore a separate worker
//! process and embedded Bun VM. The pool has no Bun dependency; it only
//! schedules protocol requests to already-isolated processes.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use super::lifecycle::RecyclePolicy;
use super::manager::{ImageLimits, ManagerError, WorkerManager};
use super::protocol::{ExecuteRequest, ExecuteResponse};
use super::sandbox::ResourceLimits;

#[derive(Debug)]
pub enum PoolError {
    Manager(ManagerError),
    NoWorkers,
    Backpressure,
    Poisoned,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manager(error) => write!(f, "worker pool manager failed: {error}"),
            Self::NoWorkers => write!(f, "worker pool has no workers"),
            Self::Backpressure => write!(f, "worker pool queue is full"),
            Self::Poisoned => write!(f, "worker pool lock is poisoned"),
        }
    }
}

impl std::error::Error for PoolError {}

impl From<ManagerError> for PoolError {
    fn from(error: ManagerError) -> Self {
        Self::Manager(error)
    }
}

struct WorkerSlot {
    manager: Mutex<WorkerManager>,
    active: AtomicUsize,
}

struct Admission {
    in_flight: Mutex<usize>,
    changed: Condvar,
    limit: usize,
}

struct AdmissionGuard {
    admission: Arc<Admission>,
}

impl Drop for AdmissionGuard {
    fn drop(&mut self) {
        if let Ok(mut in_flight) = self.admission.in_flight.lock() {
            *in_flight = in_flight.saturating_sub(1);
            self.admission.changed.notify_one();
        }
    }
}

pub struct WorkerPool {
    slots: Vec<Arc<WorkerSlot>>,
    max_in_flight: usize,
    admission: Arc<Admission>,
    recycle_policy: RecyclePolicy,
    resource_limits: ResourceLimits,
    image_limits: ImageLimits,
}

impl std::fmt::Debug for WorkerPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerPool")
            .field("workers", &self.slots.len())
            .field("max_in_flight", &self.max_in_flight)
            .field("recycle_policy", &self.recycle_policy)
            .field("resource_limits", &self.resource_limits)
            .finish()
    }
}

impl WorkerPool {
    pub fn new(
        worker_binary: PathBuf,
        socket_dir: impl Into<PathBuf>,
        worker_count: usize,
        max_in_flight: usize,
    ) -> Self {
        let socket_dir = socket_dir.into();
        let worker_count = worker_count.max(1);
        let max_in_flight = max_in_flight.max(worker_count);
        let slots = (0..worker_count)
            .map(|index| {
                let socket_path = socket_dir.join(format!("worker-{index}.sock"));
                Arc::new(WorkerSlot {
                    manager: Mutex::new(WorkerManager::new(worker_binary.clone(), socket_path)),
                    active: AtomicUsize::new(0),
                })
            })
            .collect();
        Self {
            slots,
            max_in_flight,
            admission: Arc::new(Admission {
                in_flight: Mutex::new(0),
                changed: Condvar::new(),
                limit: max_in_flight,
            }),
            recycle_policy: RecyclePolicy::disabled(),
            resource_limits: ResourceLimits::disabled(),
            image_limits: ImageLimits::default(),
        }
    }

    pub fn with_recycle_policy(mut self, policy: RecyclePolicy) -> Self {
        self.recycle_policy = policy;
        self
    }

    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = limits;
        for slot in &self.slots {
            if let Ok(mut manager) = slot.manager.lock() {
                manager.set_resource_limits(self.resource_limits.clone());
            }
        }
        self
    }

    pub fn with_image_limits(mut self, limits: ImageLimits) -> Self {
        self.image_limits = limits;
        for slot in &self.slots {
            if let Ok(mut manager) = slot.manager.lock() {
                manager.set_image_limits(self.image_limits);
            }
        }
        self
    }

    pub fn worker_count(&self) -> usize {
        self.slots.len()
    }

    /// Return the number of requests currently admitted to the pool.
    ///
    /// This is useful for readiness probes and for deterministic integration
    /// tests that need to observe admission before checking backpressure.
    pub fn in_flight(&self) -> Result<usize, PoolError> {
        self.admission
            .in_flight
            .lock()
            .map(|in_flight| *in_flight)
            .map_err(|_| PoolError::Poisoned)
    }

    pub fn start(&self) -> Result<(), PoolError> {
        for slot in &self.slots {
            slot.manager
                .lock()
                .map_err(|_| PoolError::Poisoned)?
                .start_worker()?;
        }
        Ok(())
    }

    pub fn execute(
        &self,
        request: ExecuteRequest,
        timeout_ms: u64,
    ) -> Result<ExecuteResponse, PoolError> {
        if self.slots.is_empty() {
            return Err(PoolError::NoWorkers);
        }
        let _admission = self.acquire(timeout_ms)?;

        let slot = self
            .slots
            .iter()
            .min_by_key(|slot| slot.active.load(Ordering::Acquire))
            .ok_or(PoolError::NoWorkers)?
            .clone();
        slot.active.fetch_add(1, Ordering::AcqRel);
        let dispatch_started = Instant::now();
        let remaining_timeout_ms =
            || timeout_ms.saturating_sub(dispatch_started.elapsed().as_millis() as u64);
        let result = match slot.manager.lock() {
            Ok(mut manager) => {
                if manager.should_recycle(&self.recycle_policy) {
                    if let Err(error) = manager.restart_worker() {
                        Err(PoolError::from(error))
                    } else {
                        manager
                            .execute_with_timeout(request, remaining_timeout_ms())
                            .map_err(PoolError::from)
                    }
                } else {
                    manager
                        .execute_with_timeout(request, remaining_timeout_ms())
                        .map_err(PoolError::from)
                }
            }
            Err(_) => Err(PoolError::Poisoned),
        };
        slot.active.fetch_sub(1, Ordering::AcqRel);
        result
    }

    pub fn health_check(&self) -> Result<(), PoolError> {
        for slot in &self.slots {
            slot.manager
                .lock()
                .map_err(|_| PoolError::Poisoned)?
                .health_check()?;
        }
        Ok(())
    }

    pub fn restart_all(&self) -> Result<(), PoolError> {
        for slot in &self.slots {
            slot.manager
                .lock()
                .map_err(|_| PoolError::Poisoned)?
                .restart_worker()?;
        }
        Ok(())
    }

    fn acquire(&self, timeout_ms: u64) -> Result<AdmissionGuard, PoolError> {
        let mut in_flight = self
            .admission
            .in_flight
            .lock()
            .map_err(|_| PoolError::Poisoned)?;
        let deadline = (timeout_ms > 0).then(|| Instant::now() + Duration::from_millis(timeout_ms));
        while *in_flight >= self.admission.limit {
            if let Some(deadline) = deadline {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(PoolError::Backpressure);
                }
                let (next, timeout) = self
                    .admission
                    .changed
                    .wait_timeout(in_flight, remaining)
                    .map_err(|_| PoolError::Poisoned)?;
                in_flight = next;
                if timeout.timed_out() && *in_flight >= self.admission.limit {
                    return Err(PoolError::Backpressure);
                }
            } else {
                in_flight = self
                    .admission
                    .changed
                    .wait(in_flight)
                    .map_err(|_| PoolError::Poisoned)?;
            }
        }
        *in_flight += 1;
        Ok(AdmissionGuard {
            admission: Arc::clone(&self.admission),
        })
    }
}
