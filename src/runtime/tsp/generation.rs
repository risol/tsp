//! Generation + PageSlot + PageState for TSP v2 slice 10a
//! (plan sect.20.3-20.4 + sect.21).
//!
//! See `tsp-v2-plan.md` sect.20.3 (PageSlot) and sect.21
//! (Generation + atomic publish). This file lands the data
//! structures + the state machine. The actual transpile +
//! evaluate + publish (sect.21.1) lands in slice 10b; the
//! in-flight dedup and request pinning land in slice 10c.
//!
//! Scope for slice 10a:
//! - `GenerationId` (monotonic u64) and `Generation` struct.
//! - `PageState` enum (Unloaded, Clean, Dirty, Building, Failed).
//! - `PageSlot` with `current` + `last_known_good` Generation
//!   fields, plus the state.
//! - `PageRegistry` that owns all slots and serialises state
//!   transitions. Cheap to clone (`Arc<Mutex<RegistryInner>>`)
//!   so the binary can hold a registry handle and the
//!   `PublishGuard` can hold an independent handle into the
//!   same state.
//! - State machine: `mark_dirty`, `begin_build`,
//!   `PublishGuard::commit(Ok)` / `commit(Failed(_))`, and the
//!   LKG roll-back path.
//! - Free function `new_generation_id` (atomic) so the registry
//!   never re-uses a GenerationId.
//!
//! Out of slice 10a:
//! - The transpile + JSC evaluate step (slice 10b). The
//!   registry does not know how to build a candidate; it only
//!   tracks the state.
//! - The watcher-driven dirty marking (slice 11).
//! - Request pinning + in-flight dedup (slice 10c).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::module_graph::ModuleId;
use crate::router::HttpMethod;

#[derive(Debug, Clone)]
pub struct Generation {
    pub id: GenerationId,
    /// The PageRef this generation serves. Each generation is
    /// per-page (not per-graph); two pages do not share a
    /// generation even if they share a base module.
    pub page: PageRef,
    /// Dependencies this generation pulled in. Used by the
    /// watcher (slice 11) to decide whether an unrelated dep
    /// change invalidates the generation.
    pub dependencies: Vec<ModuleId>,
    /// Wall-clock time at which this generation was published.
    pub created_at: std::time::Instant,
    /// Result of the build (Ok or the first error). Empty for
    /// the placeholder; populated when the candidate completes.
    pub build_result: BuildResult,
    /// The rendered HTTP body for the page handler. `None`
    /// for `Failed` generations and for the placeholder
    /// before `commit`; `Some(body)` for `Ok` generations.
    /// The host reads this directly to avoid a per-request
    /// re-build.
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenerationId(u64);

/// Monotonic counter for `GenerationId`.
static GENERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Allocate a fresh `GenerationId`. Never re-uses an id within the
/// process lifetime.
pub fn new_generation_id() -> GenerationId {
    GenerationId(GENERATION_COUNTER.fetch_add(1, Ordering::Relaxed) + 1)
}

impl GenerationId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildResult {
    /// The candidate built and passed export validation. The
    /// runtime can serve requests from this generation.
    Ok,
    /// The candidate failed at one of the build steps (resolve,
    /// transpile, evaluate, validate). The message is bubbled up
    /// so the dev error page / production 500 can surface it.
    Failed(String),
}

/// Reference to a route in the registry. Distinct from
/// `module_graph::PageId` so the registry can hold per-page
/// state without the module-graph layer being involved in the
/// request hot path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PageRef {
    /// The URL path the route is mounted at. E.g. "/" for
    /// `routes/index.tsp`.
    pub route: String,
    /// The HTTP method the slot dispatches. A `.tsp` file
    /// that exports both `GET` and `POST` becomes two
    /// `PageSlot`s sharing the same source module but with
    /// different `method` values (per plan sect.4.2).
    pub method: HttpMethod,
}

/// Per-plan: "Unloaded, Clean, Dirty, Building, Failed".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageState {
    /// The page has never built successfully. The first
    /// request triggers a `begin_build`. If the build
    /// fails, the slot stays in `Failed` (or returns 500
    /// per plan sect.24.2).
    Unloaded,
    /// The slot has a `current` generation and no pending
    /// changes. Requests serve from `current`.
    Clean,
    /// A watched file changed; the slot needs a rebuild. The
    /// rebuild is lazy: it happens on the next request
    /// (plan sect.22.2).
    Dirty,
    /// A `begin_build` is in flight. Concurrent requests
    /// either await the same build (dev) or stay on the
    /// LKG generation (production, plan sect.22.4).
    Building,
    /// The most recent `begin_build` finished with a
    /// `BuildResult::Failed`. `current` still points at
    /// the LKG (which may be `None` for a never-loaded
    /// page).
    Failed,
}

/// One slot per (route, method) pair.
#[derive(Debug)]
pub struct PageSlot {
    pub page: PageRef,
    /// The module the slot loads from. Same across all
    /// methods of a `.tsp` file.
    pub source: ModuleId,
    /// Currently-served generation. `None` only when state is
    /// `Unloaded` or the first build is still pending.
    pub current: Option<Generation>,
    /// Last successful generation. Used when `current` is
    /// missing (first-load failure) or when a dirty rebuild
    /// fails (production serves LKG, plan sect.24.1).
    pub last_known_good: Option<Generation>,
    /// Build state.
    pub state: PageState,
}

impl PageSlot {
    /// Build the initial empty slot for a (route, method)
    /// pair, before any successful load.
    pub fn new_unloaded(page: PageRef, source: ModuleId) -> Self {
        Self {
            page,
            source,
            current: None,
            last_known_good: None,
            state: PageState::Unloaded,
        }
    }

    /// True when the slot can serve requests from `current`
    /// without triggering a build.
    pub fn is_servable(&self) -> bool {
        matches!(self.state, PageState::Clean | PageState::Failed | PageState::Building)
            && self.current.is_some()
    }
}

/// Cheap-to-clone handle into the page registry. The
/// `Arc<Mutex<RegistryInner>>` lets `PublishGuard` hold an
/// independent handle without violating aliasing.
#[derive(Debug, Clone)]
pub struct PageRegistry {
    inner: Arc<Mutex<RegistryInner>>,
}

#[derive(Debug, Default)]
struct RegistryInner {
    /// Keyed by the canonical PageRef (route + method). Two
    /// methods of the same file become two entries; the
    /// `source` field is identical between them so the
    /// generation pipeline can share work.
    slots: HashMap<PageRef, PageSlot>,
    /// Generation publish ordering.
    generation_log: Vec<GenerationId>,
}

impl Default for PageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PageRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RegistryInner::default())),
        }
    }

    /// Register a page so the registry can later look it up by
    /// `PageRef`. Idempotent: a second call for the same
    /// `PageRef` is a no-op (the existing slot is kept,
    /// including its state).
    pub fn register(&self, page: PageRef, source: ModuleId) {
        let mut inner = self.inner.lock().expect("registry lock poisoned");
        inner
            .slots
            .entry(page.clone())
            .or_insert_with(|| PageSlot::new_unloaded(page, source));
    }

    /// Look up a slot by page reference. Returns a snapshot
    /// of the slot's `state` + `current` / `last_known_good`
    /// ids so the request hot path can decide "serve from
    /// current" / "serve from LKG" / "trigger build" without
    /// holding the lock.
    pub fn snapshot(&self, page: &PageRef) -> Option<SlotSnapshot> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        let slot = inner.slots.get(page)?;
        Some(SlotSnapshot {
            state: slot.state.clone(),
            current_id: slot.current.as_ref().map(|g| g.id),
            last_known_good_id: slot.last_known_good.as_ref().map(|g| g.id),
        })
    }

    /// Read the current generation's payload (the rendered
    /// HTTP body). Clones the string; the lock is held only
    /// for the duration of the clone. Returns `None` if the
    /// slot is not in the registry, has no `current`, or the
    /// `current` is a `Failed` build (no payload).
    pub fn read_current_payload(&self, page: &PageRef) -> Option<String> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        let slot = inner.slots.get(page)?;
        slot.current.as_ref().and_then(|g| g.payload.clone())
    }

    /// Read the LKG generation's payload. Same shape as
    /// `read_current_payload`.
    pub fn read_lkg_payload(&self, page: &PageRef) -> Option<String> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        let slot = inner.slots.get(page)?;
        slot.last_known_good.as_ref().and_then(|g| g.payload.clone())
    }

    /// Mark a slot dirty. Used by the watcher (slice 11) and
    /// the request hot path (when a request arrives for a
    /// `Clean` slot that the watcher already flagged).
    ///
    /// Idempotent: a Dirty -> Dirty transition is a no-op.
    /// Building -> Dirty is rejected; the in-flight build must
    /// either complete (Clean) or fail (Failed) before a new
    /// dirty is allowed.
    pub fn mark_dirty(&self, page: &PageRef) -> MarkDirtyResult {
        let mut inner = self.inner.lock().expect("registry lock poisoned");
        let Some(slot) = inner.slots.get_mut(page) else {
            return MarkDirtyResult::UnknownPage;
        };
        match slot.state {
            PageState::Unloaded => MarkDirtyResult::AlreadyFirstLoad,
            PageState::Clean => {
                slot.state = PageState::Dirty;
                MarkDirtyResult::Marked
            }
            PageState::Dirty => MarkDirtyResult::AlreadyDirty,
            PageState::Building => MarkDirtyResult::BuildInFlight,
            PageState::Failed => {
                slot.state = PageState::Dirty;
                MarkDirtyResult::Marked
            }
        }
    }

    /// Begin a build. Returns a `PublishGuard` if the slot
    /// accepts the build; `Err(NotBuildable)` otherwise. The
    /// guard hands a fresh `Generation` back to the builder,
    /// and `guard.commit(Ok)` or `guard.commit(Failed(_))`
    /// transitions the slot.
    ///
    /// Allowed transitions:
    /// - `Unloaded -> Building` (first load)
    /// - `Dirty -> Building` (rebuild)
    /// - `Failed -> Building` (retry after failure)
    ///
    /// Disallowed:
    /// - `Clean -> Building` (no reason to rebuild)
    /// - `Building -> Building` (in-flight)
    pub fn begin_build(&self, page: &PageRef) -> Result<PublishGuard, BeginBuildError> {
        let mut inner = self.inner.lock().expect("registry lock poisoned");
        let Some(slot) = inner.slots.get_mut(page) else {
            return Err(BeginBuildError::UnknownPage);
        };
        match slot.state {
            PageState::Unloaded | PageState::Dirty | PageState::Failed => {
                let id = new_generation_id();
                let candidate = Generation {
                    id,
                    page: slot.page.clone(),
                    dependencies: Vec::new(),
                    created_at: std::time::Instant::now(),
                    // Placeholder; `commit` overwrites with
                    // `Ok` and fills `payload`. The `Failed`
                    // arm also overwrites `build_result`
                    // but leaves `payload` as None.
                    build_result: BuildResult::Failed(String::new()),
                    payload: None,
                };
                slot.state = PageState::Building;
                let slot_page = slot.page.clone();
                inner.generation_log.push(candidate.id);
                Ok(PublishGuard {
                    registry: self.clone(),
                    slot_page,
                    candidate: Some(candidate),
                })
            }
            PageState::Clean => Err(BeginBuildError::NotBuildable(PageState::Clean)),
            PageState::Building => Err(BeginBuildError::NotBuildable(PageState::Building)),
        }
    }
}

/// Result of `mark_dirty`.
#[derive(Debug, PartialEq, Eq)]
pub enum MarkDirtyResult {
    /// Slot transitioned Clean / Failed -> Dirty.
    Marked,
    /// Slot was already Dirty. No-op.
    AlreadyDirty,
    /// Slot is Building. The in-flight build owns the
    /// outcome.
    BuildInFlight,
    /// Page is in its first load (Unloaded).
    AlreadyFirstLoad,
    /// Page is not in the registry.
    UnknownPage,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BeginBuildError {
    UnknownPage,
    NotBuildable(PageState),
}

/// Snapshot the request hot path reads without holding the
/// registry lock.
#[derive(Debug, Clone)]
pub struct SlotSnapshot {
    pub state: PageState,
    pub current_id: Option<GenerationId>,
    pub last_known_good_id: Option<GenerationId>,
}

/// RAII guard returned by `begin_build`. Holds the candidate
/// generation; the builder fills in `dependencies` and the
/// `build_result` then calls `commit(Ok)` or
/// `commit(Failed(msg))`. If the guard is dropped without
/// commit (e.g. the builder panics), the slot is reset to its
/// pre-build state (Failed or Dirty).
#[derive(Debug)]
pub struct PublishGuard {
    registry: PageRegistry,
    slot_page: PageRef,
    candidate: Option<Generation>,
}

impl PublishGuard {
    /// Fill in the dependency list captured during the
    /// transpile + evaluate step. Idempotent (last call wins).
    pub fn record_dependencies(&mut self, deps: Vec<ModuleId>) {
        if let Some(c) = self.candidate.as_mut() {
            c.dependencies = deps;
        }
    }

    /// Commit a successful build. Promotes the candidate to
    /// `current`. The LKG semantics is "the last successful
    /// build":
    /// - First load: `current` was None, so LKG = candidate.
    /// - Subsequent successful commit: `current` was a previous
    ///   successful build, so LKG = that previous build and
    ///   `current` = candidate.
    /// - The `failed-then-succeed` case (prev was Failed):
    ///   LKG = candidate (the new successful build is the
    ///   fallback for the next failure).
    pub fn commit(mut self, deps: Vec<ModuleId>, payload: String) {
        let mut candidate = self.candidate.take().expect("candidate already committed");
        candidate.dependencies = deps;
        candidate.build_result = BuildResult::Ok;
        candidate.payload = Some(payload);
        let mut inner = self.registry.inner.lock().expect("registry lock poisoned");
        if let Some(slot) = inner.slots.get_mut(&self.slot_page) {
            let prev = slot.current.take();
            let prev_was_ok = matches!(prev, Some(ref p) if matches!(p.build_result, BuildResult::Ok));
            if prev_was_ok {
                // Promote the previous successful current to
                // LKG; the new candidate takes over `current`.
                slot.last_known_good = prev;
            } else {
                // No previous Ok build (first load, or
                // previous was Failed). LKG = the new
                // candidate so a future failure has a
                // fallback.
                slot.last_known_good = Some(candidate.clone());
            }
            slot.current = Some(candidate);
            slot.state = PageState::Clean;
        }
    }

    /// Commit a failed build. The candidate becomes a
    /// `Failed` generation we keep for diagnostics, but is
    /// NOT promoted to `current`. State becomes `Failed`.
    pub fn fail(mut self, message: String) {
        let mut candidate = self.candidate.take().expect("candidate already committed");
        candidate.build_result = BuildResult::Failed(message);
        let mut inner = self.registry.inner.lock().expect("registry lock poisoned");
        if let Some(slot) = inner.slots.get_mut(&self.slot_page) {
            // Do NOT promote to current; do NOT overwrite LKG.
            // The previous `current` (if it was a successful
            // build) stays as `current` so requests still
            // serve from it. Per plan sect.24.2 a never-loaded
            // page that fails its first build returns 500.
            slot.state = PageState::Failed;
        }
    }
}

impl std::ops::Drop for PublishGuard {
    fn drop(&mut self) {
        // If the guard is dropped without commit / fail, the
        // build was abandoned. Reset state so the next
        // request can re-trigger.
        if self.candidate.is_some() {
            let mut inner = self.registry.inner.lock().expect("registry lock poisoned");
            if let Some(slot) = inner.slots.get_mut(&self.slot_page) {
                // The previous transition was into Building;
                // roll back to Dirty (or Unloaded for the
                // first-load case) so the next request can
                // try again.
                slot.state = if slot.current.is_none() {
                    PageState::Unloaded
                } else {
                    PageState::Dirty
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn page(route: &str, method: HttpMethod) -> PageRef {
        PageRef {
            route: route.to_string(),
            method,
        }
    }
    fn modid(s: &str) -> ModuleId {
        ModuleId::from_canonical_path(PathBuf::from(s))
    }

    #[test]
    fn first_load_unloaded_to_building_to_clean() {
        let r = PageRegistry::new();
        let p = page("/", HttpMethod::Get);
        r.register(p.clone(), modid("routes/index.tsp"));
        let snap = r.snapshot(&p).unwrap();
        assert_eq!(snap.state, PageState::Unloaded);
        assert!(snap.current_id.is_none());
        let guard = r.begin_build(&p).unwrap();
        let snap = r.snapshot(&p).unwrap();
        assert_eq!(snap.state, PageState::Building);
        guard.commit(vec![modid("routes/index.tsp")], "<h1>Hello</h1>".to_string());
        let snap = r.snapshot(&p).unwrap();
        assert_eq!(snap.state, PageState::Clean);
        assert!(snap.current_id.is_some());
    }

    #[test]
    fn first_load_failure_keeps_no_current() {
        let r = PageRegistry::new();
        let p = page("/", HttpMethod::Get);
        r.register(p.clone(), modid("routes/index.tsp"));
        let guard = r.begin_build(&p).unwrap();
        guard.fail("transpile error".into());
        let snap = r.snapshot(&p).unwrap();
        assert_eq!(snap.state, PageState::Failed);
        assert!(snap.current_id.is_none());
    }

    #[test]
    fn clean_to_dirty_to_building_to_clean_lkg_promotes() {
        let r = PageRegistry::new();
        let p = page("/", HttpMethod::Get);
        r.register(p.clone(), modid("routes/index.tsp"));
        // First build succeeds
        let g1 = r.begin_build(&p).unwrap();
        g1.commit(vec![], "body1".to_string());
        let snap = r.snapshot(&p).unwrap();
        let first = snap.current_id.unwrap();
        // Mark dirty
        assert_eq!(r.mark_dirty(&p), MarkDirtyResult::Marked);
        // Second build succeeds
        let g2 = r.begin_build(&p).unwrap();
        g2.commit(vec![], "body2".to_string());
        let snap = r.snapshot(&p).unwrap();
        assert_eq!(snap.state, PageState::Clean);
        assert!(snap.current_id.is_some());
        assert_ne!(snap.current_id.unwrap(), first);
        // LKG should be the first build
        assert_eq!(snap.last_known_good_id, Some(first));
    }

    #[test]
    fn failed_rebuild_keeps_lkg() {
        let r = PageRegistry::new();
        let p = page("/", HttpMethod::Get);
        r.register(p.clone(), modid("routes/index.tsp"));
        // First build succeeds. Per "LKG = last successful
        // build", after the first commit LKG = g1 and
        // current = g1.
        let g1 = r.begin_build(&p).unwrap();
        g1.commit(vec![], "body1".to_string());
        let first = r.snapshot(&p).unwrap().current_id.unwrap();
        assert_eq!(r.snapshot(&p).unwrap().last_known_good_id, Some(first));
        // Mark dirty and try a second build that fails.
        r.mark_dirty(&p);
        let g2 = r.begin_build(&p).unwrap();
        g2.fail("boom".into());
        let snap = r.snapshot(&p).unwrap();
        assert_eq!(snap.state, PageState::Failed);
        // `current` is still the first build (fail doesn't
        // replace it); LKG is still the first build.
        assert_eq!(snap.current_id, Some(first));
        assert_eq!(snap.last_known_good_id, Some(first));
    }

    #[test]
    fn dropped_guard_resets_state() {
        let r = PageRegistry::new();
        let p = page("/", HttpMethod::Get);
        r.register(p.clone(), modid("routes/index.tsp"));
        let g1 = r.begin_build(&p).unwrap();
        // Drop without commit.
        drop(g1);
        // State should not be stuck in Building.
        let snap = r.snapshot(&p).unwrap();
        assert_eq!(snap.state, PageState::Unloaded);
    }

    #[test]
    fn begin_build_rejects_clean_and_building() {
        let r = PageRegistry::new();
        let p = page("/", HttpMethod::Get);
        r.register(p.clone(), modid("routes/index.tsp"));
        let g = r.begin_build(&p).unwrap();
        g.commit(vec![], "body".to_string());
        // Now Clean. begin_build must be rejected.
        let err = r.begin_build(&p).unwrap_err();
        assert_eq!(err, BeginBuildError::NotBuildable(PageState::Clean));
        // Mark dirty then begin_build succeeds.
        r.mark_dirty(&p);
        let g2 = r.begin_build(&p).unwrap();
        g2.commit(vec![], "body2".to_string());
    }

    #[test]
    fn generation_id_is_monotonic() {
        let a = new_generation_id();
        let b = new_generation_id();
        assert!(b.as_u64() > a.as_u64());
    }

    #[test]
    fn unknown_page_returns_error() {
        let r = PageRegistry::new();
        let p = page("/missing", HttpMethod::Get);
        let err = r.begin_build(&p).unwrap_err();
        assert_eq!(err, BeginBuildError::UnknownPage);
    }

    #[test]
    fn registry_is_cheap_to_clone() {
        let r1 = PageRegistry::new();
        let r2 = r1.clone();
        let p = page("/", HttpMethod::Get);
        r1.register(p.clone(), modid("routes/index.tsp"));
        // Both handles see the same slot.
        assert!(r1.snapshot(&p).is_some());
        assert!(r2.snapshot(&p).is_some());
    }
}