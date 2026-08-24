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
use std::sync::{Arc, Condvar, Mutex};

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
    ///
    /// `Arc<String>` so concurrent requests share the same
    /// buffer (plan sect.21.3 request pinning) and a request
    /// that pinned the body before a later commit can still
    /// finish on the generation it observed. The Arc keeps
    /// the body alive even after `current` is overwritten.
    pub payload: Option<Arc<String>>,
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
    /// Shared build future, set when `state == Building`.
    /// Concurrent requests on a Building slot wait on this
    /// (plan sect.22.4 in-flight dedup). `None` outside of a
    /// build.
    pub in_flight: Option<Arc<InFlightBuild>>,
}

impl std::fmt::Debug for PageSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageSlot")
            .field("page", &self.page)
            .field("source", &self.source)
            .field("current_id", &self.current.as_ref().map(|g| g.id))
            .field("last_known_good_id", &self.last_known_good.as_ref().map(|g| g.id))
            .field("state", &self.state)
            .field("in_flight", &self.in_flight.as_ref().map(|_| "Arc<InFlightBuild>"))
            .finish()
    }
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
            in_flight: None,
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

    /// Remove a slot. Used by the watcher (slice 15a) when a
    /// `.tsp` file disappears -- the route goes away from the
    /// `RouteTable`, and the corresponding `PageSlot` must go
    /// away from the `PageRegistry` too. Idempotent: removing
    /// a non-existent slot is a no-op. Returns `true` if a
    /// slot was actually removed, `false` otherwise.
    pub fn unregister(&self, page: &PageRef) -> bool {
        let mut inner = self.inner.lock().expect("registry lock poisoned");
        inner.slots.remove(page).is_some()
    }

    /// Register a page from a single `Route` (the watcher's
    /// slice-15a add path). Reads the source, runs the slice 5
    /// detector, and registers one slot per HTTP method the
    /// file exports. This is the runtime side of the slice 3
    /// boot-time `build_registry`; both call the same
    /// `page::prepare` so the methods list stays consistent.
    pub fn register_route(&self, route: &crate::router::Route) {
        let source = crate::module_graph::ModuleId::from_path(&route.source);
        for &method in &route.methods {
            let page_ref = PageRef {
                route: route.path.clone(),
                method,
            };
            self.register(page_ref, source.clone());
        }
    }

    /// Unregister every slot whose `route` field matches the
    /// given URL path. Used by the watcher (slice 15a) when a
    /// `.tsp` file disappears. The route has multiple slots
    /// (one per HTTP method), all of which must go away.
    /// Returns the number of slots removed.
    pub fn unregister_path(&self, route_path: &str) -> usize {
        let mut inner = self.inner.lock().expect("registry lock poisoned");
        let before = inner.slots.len();
        inner.slots.retain(|page_ref, _| page_ref.route != route_path);
        before - inner.slots.len()
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

    /// List every registered `PageRef`. Slice 11's watcher uses
    /// this to mark all slots dirty on any file change (the
    /// precise source->PageRef index lands in slice 12); the
    /// dev-inspector (future slice) uses it to render the page
    /// table.
    pub fn all_page_refs(&self) -> Vec<PageRef> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        inner.slots.keys().cloned().collect()
    }

    /// Read the current generation's payload (the rendered
    /// HTTP body). Clones the inner `String`; the lock is
    /// held only for the duration of the clone. Returns
    /// `None` if the slot is not in the registry, has no
    /// `current`, or the `current` is a `Failed` build (no
    /// payload).
    ///
    /// Prefer `read_current_arc` for the request hot path
    /// (plan sect.21.3 request pinning) -- this String-clone
    /// form is kept for tests and for callers that need a
    /// one-off copy.
    pub fn read_current_payload(&self, page: &PageRef) -> Option<String> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        let slot = inner.slots.get(page)?;
        slot.current.as_ref().and_then(|g| g.payload.as_ref().map(|a| (**a).clone()))
    }

    /// Read the LKG generation's payload. Same shape as
    /// `read_current_payload`.
    pub fn read_lkg_payload(&self, page: &PageRef) -> Option<String> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        let slot = inner.slots.get(page)?;
        slot.last_known_good.as_ref().and_then(|g| g.payload.as_ref().map(|a| (**a).clone()))
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
    /// - `Building -> Building` (in-flight; use
    ///   `join_in_flight` instead -- plan sect.22.4)
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
                // Create the shared in-flight build so concurrent
                // requests can wait on the same future.
                let shared = Arc::new(InFlightBuild {
                    page: slot.page.clone(),
                    state: Mutex::new(InFlightState::Running),
                    cvar: Condvar::new(),
                });
                slot.state = PageState::Building;
                slot.in_flight = Some(shared.clone());
                let slot_page = slot.page.clone();
                inner.generation_log.push(candidate.id);
                Ok(PublishGuard {
                    registry: self.clone(),
                    slot_page,
                    candidate: Some(candidate),
                    shared: Some(shared),
                })
            }
            PageState::Clean => Err(BeginBuildError::NotBuildable(PageState::Clean)),
            PageState::Building => Err(BeginBuildError::NotBuildable(PageState::Building)),
        }
    }

    /// Return the in-flight build for a slot that is currently
    /// in the `Building` state, or `None` otherwise. Callers
    /// that find a `Building` slot during the request hot path
    /// use this to wait on the same future (plan sect.22.4 in-
    /// flight dedup). The future can be `wait`ed on for the
    /// outcome.
    pub fn join_in_flight(&self, page: &PageRef) -> Option<Arc<InFlightBuild>> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        inner.slots.get(page).and_then(|s| s.in_flight.clone())
    }

    /// Pin the current generation's body. Returns an `Arc<String>`
    /// the caller can keep alive across a later `commit`. This
    /// is the request-pinning primitive: even if the next
    /// commit overwrites `current`, the pinned `Arc<String>`
    /// still resolves to the body observed at pin time
    /// (plan sect.21.3).
    pub fn read_current_arc(&self, page: &PageRef) -> Option<Arc<String>> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        let slot = inner.slots.get(page)?;
        slot.current.as_ref().and_then(|g| g.payload.clone())
    }

    /// Pin the LKG generation's body. Same shape as
    /// `read_current_arc`; used by the request hot path when
    /// `current` is missing or the in-flight build failed.
    pub fn read_lkg_arc(&self, page: &PageRef) -> Option<Arc<String>> {
        let inner = self.inner.lock().expect("registry lock poisoned");
        let slot = inner.slots.get(page)?;
        slot.last_known_good.as_ref().and_then(|g| g.payload.clone())
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
///
/// `shared` is the `InFlightBuild` the registry stored on the
/// slot. `commit` / `fail` write the outcome into it and
/// notify waiters; `drop` writes `Abandoned` so the waiters
/// wake up and fall back to LKG (or 500).
pub struct PublishGuard {
    registry: PageRegistry,
    slot_page: PageRef,
    candidate: Option<Generation>,
    shared: Option<Arc<InFlightBuild>>,
}

impl std::fmt::Debug for PublishGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublishGuard")
            .field("slot_page", &self.slot_page)
            .field("has_candidate", &self.candidate.is_some())
            .field("has_shared", &self.shared.is_some())
            .finish()
    }
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
        let arc_payload: Arc<String> = Arc::new(payload);
        candidate.payload = Some(arc_payload.clone());
        // Finalize the in-flight future BEFORE we touch the
        // slot state, so a waiter waking up between the write
        // and the slot transition still sees a valid outcome
        // (the condvar is paired with the InFlightBuild, not
        // the slot). The shared build is consumed here -- the
        // waiter only needs its outcome, not the InFlightBuild
        // itself.
        if let Some(shared) = self.shared.take() {
            let mut guard = shared.state.lock().expect("in-flight lock poisoned");
            *guard = InFlightState::Done(BuildOutcome::Ok(arc_payload));
            shared.cvar.notify_all();
            // InFlightBuild is dropped here; the waiters keep
            // their own Arc<InFlightBuild> only if they
            // cloned it before commit; in practice they only
            // need the outcome, which they read once.
        }
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
            slot.in_flight = None;
        }
    }

    /// Commit a failed build. The candidate becomes a
    /// `Failed` generation we keep for diagnostics, but is
    /// NOT promoted to `current`. State becomes `Failed`.
    pub fn fail(mut self, message: String) {
        let mut candidate = self.candidate.take().expect("candidate already committed");
        candidate.build_result = BuildResult::Failed(message.clone());
        if let Some(shared) = self.shared.take() {
            let mut guard = shared.state.lock().expect("in-flight lock poisoned");
            *guard = InFlightState::Done(BuildOutcome::Failed(message));
            shared.cvar.notify_all();
        }
        let mut inner = self.registry.inner.lock().expect("registry lock poisoned");
        if let Some(slot) = inner.slots.get_mut(&self.slot_page) {
            // Do NOT promote to current; do NOT overwrite LKG.
            // The previous `current` (if it was a successful
            // build) stays as `current` so requests still
            // serve from it. Per plan sect.24.2 a never-loaded
            // page that fails its first build returns 500.
            slot.state = PageState::Failed;
            slot.in_flight = None;
        }
    }
}

impl std::ops::Drop for PublishGuard {
    fn drop(&mut self) {
        // If the guard is dropped without commit / fail, the
        // build was abandoned. Reset state so the next
        // request can re-trigger, and wake any waiters so
        // they fall back to LKG instead of waiting forever.
        if self.candidate.is_some() {
            if let Some(shared) = self.shared.take() {
                let mut guard = shared.state.lock().expect("in-flight lock poisoned");
                if matches!(*guard, InFlightState::Running) {
                    *guard = InFlightState::Abandoned;
                    shared.cvar.notify_all();
                }
            }
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
                slot.in_flight = None;
            }
        }
    }
}

/// A build that is currently in flight. Concurrent requests on
/// the same `Building` slot share the same `Arc<InFlightBuild>`
/// and wait on `cvar` for the outcome (plan sect.22.4 in-flight
/// dedup). The owner thread fills the outcome in via
/// `PublishGuard::commit` or `fail`; the drop path fills
/// `Abandoned` so a panic never leaves waiters stuck.
pub struct InFlightBuild {
    pub page: PageRef,
    /// The `cvar` is paired with this `state` lock.
    /// `pub` so the host can lock it directly to wait via
    /// `wait()`; the lock-and-wait idiom is the standard
    /// Condvar pattern.
    pub state: Mutex<InFlightState>,
    pub cvar: Condvar,
}

impl std::fmt::Debug for InFlightBuild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.state.lock().expect("in-flight lock poisoned");
        f.debug_struct("InFlightBuild")
            .field("page", &self.page)
            .field("state", &*state)
            .finish()
    }
}

/// The lifecycle of an `InFlightBuild`.
#[derive(Debug, Clone)]
pub enum InFlightState {
    /// Owner is still building. Waiters block on the condvar.
    Running,
    /// Owner finished. `BuildOutcome` is the result.
    Done(BuildOutcome),
    /// Owner dropped the guard without commit/fail (panic,
    /// unwind). Waiters should fall back to LKG / 500.
    Abandoned,
}

/// What an `InFlightBuild` resolved to.
#[derive(Debug, Clone)]
pub enum BuildOutcome {
    /// Build succeeded; `Arc<String>` is the rendered body.
    /// The `Arc` keeps the body alive even if the slot's
    /// `current` is later overwritten.
    Ok(Arc<String>),
    /// Build failed. The string is the human-readable
    /// diagnostic; the request hot path serves LKG or 500.
    Failed(String),
}

impl InFlightBuild {
    /// Block on the condvar until the outcome is known. Caller
    /// passes the lock guard it holds over `state`. Returns
    /// the final `InFlightState`. Used by request threads
    /// that joined an in-flight build and need to wait for
    /// the same future.
    pub fn wait(&self, guard: std::sync::MutexGuard<'_, InFlightState>) -> InFlightState {
        let mut guard = guard;
        while matches!(*guard, InFlightState::Running) {
            guard = self.cvar.wait(guard).expect("in-flight cvar poisoned");
        }
        guard.clone()
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

    // --- Slice 12 tests (plan sect.21.3 + 22.4) ---

    /// Concurrent requests on a Building slot see the same
    /// in-flight future and resolve to the same outcome. This
    /// is the in-flight dedup primitive.
    #[test]
    fn in_flight_dedup_shares_one_future() {
        use std::thread;
        let r = PageRegistry::new();
        let p = page("/", HttpMethod::Get);
        r.register(p.clone(), modid("routes/index.tsp"));

        // Thread A: owns the build, sleeps a bit, then commits.
        let r_a = r.clone();
        let p_a = p.clone();
        let a = thread::spawn(move || {
            let guard = r_a.begin_build(&p_a).expect("begin");
            let shared = r_a.join_in_flight(&p_a).expect("in-flight exists");
            // Sleep so the waiter has a chance to start.
            thread::sleep(std::time::Duration::from_millis(50));
            guard.commit(vec![], "A-body".to_string());
            shared
        });

        // Thread B: arrives while A is building. Joins the
        // in-flight and waits on the condvar.
        let r_b = r.clone();
        let p_b = p.clone();
        let b = thread::spawn(move || {
            // Spin until the slot is in Building.
            for _ in 0..100 {
                if matches!(
                    r_b.snapshot(&p_b).map(|s| s.state),
                    Some(PageState::Building)
                ) {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(2));
            }
            let shared = r_b.join_in_flight(&p_b).expect("in-flight exists");
            let guard = shared.state.lock().expect("lock");
            let outcome = shared.wait(guard);
            outcome
        });

        let outcome_b = b.join().expect("B join");
        let _shared_a = a.join().expect("A join");

        // B should have observed the same Ok outcome that A
        // committed.
        match outcome_b {
            InFlightState::Done(BuildOutcome::Ok(arc)) => assert_eq!(*arc, "A-body"),
            other => panic!("B expected Ok(A-body), got {other:?}"),
        }
        // After both threads, slot is Clean and in_flight is None.
        let snap = r.snapshot(&p).unwrap();
        assert_eq!(snap.state, PageState::Clean);
        assert!(r.join_in_flight(&p).is_none());
    }

    /// A request that pins the current Arc<String> before a
    /// later commit finishes on the pinned body, not on the
    /// new current. This is request pinning (plan sect.21.3).
    #[test]
    fn request_pinning_survives_commit_overwrite() {
        let r = PageRegistry::new();
        let p = page("/", HttpMethod::Get);
        r.register(p.clone(), modid("routes/index.tsp"));
        let g1 = r.begin_build(&p).unwrap();
        g1.commit(vec![], "v1".to_string());
        // Pin v1.
        let pinned = r.read_current_arc(&p).expect("current exists");
        assert_eq!(*pinned, "v1");
        // Mark dirty and rebuild.
        r.mark_dirty(&p);
        let g2 = r.begin_build(&p).unwrap();
        g2.commit(vec![], "v2".to_string());
        // Pinned v1 still resolves to "v1"; the new current
        // is v2.
        assert_eq!(*pinned, "v1");
        let new_current = r.read_current_arc(&p).expect("new current");
        assert_eq!(*new_current, "v2");
    }

    /// When `current` is overwritten, the previous Arc<String>
    /// becomes eligible for drop if no one is pinning it. We
    /// verify by checking that the buffer we held is the only
    /// one alive at pin time and that dropping it does not
    /// affect the new current.
    #[test]
    fn generation_release_drops_old_payload() {
        let r = PageRegistry::new();
        let p = page("/", HttpMethod::Get);
        r.register(p.clone(), modid("routes/index.tsp"));
        let g1 = r.begin_build(&p).unwrap();
        g1.commit(vec![], "release-me".to_string());
        // Capture a strong count via Arc::strong_count before
        // overwrite. We hold no extra clones of our own, so
        // strong_count should be 1 (only the slot owns it).
        // First commit puts the new Generation into BOTH
        // `current` and `last_known_good` (LKG semantics:
        // "last successful build"). So the Arc<String>
        // payload has 2 holders (current + LKG) + our read
        // clone = 3.
        let before = r.read_current_arc(&p).unwrap();
        let strong_before = std::sync::Arc::strong_count(&before);
        assert_eq!(strong_before, 3, "current (1) + LKG (1) + our read (1) = 3");
        // Drop our handle; the slot still owns it.
        drop(before);
        let only_slot = r.read_current_arc(&p).unwrap();
        // current + LKG + our read = 3.
        assert_eq!(std::sync::Arc::strong_count(&only_slot), 3);
        drop(only_slot);
        // Overwrite: the slot's strong count should drop to 0
        // for the old buffer and 2 for the new one.
        r.mark_dirty(&p);
        let g2 = r.begin_build(&p).unwrap();
        g2.commit(vec![], "fresh".to_string());
        let after = r.read_current_arc(&p).unwrap();
        assert_eq!(*after, "fresh");
        // Second commit: prev successful was promoted to LKG
        // (it stays), new goes to current. So the new
        // Arc<String> has current (1) + LKG-clone-doesn't-
        // apply-here-no (1) + our read (1) = 2 -- wait, the
        // new commit's prev_was_ok path is `slot.last_known_good
        // = prev` which moves (not clones) the previous
        // Generation. So the new candidate Arc has just
        // current (1) + our read (1) = 2.
        assert_eq!(std::sync::Arc::strong_count(&after), 2);
    }

    /// A request that joins a Building slot and the build
    /// fails: the waiter sees the Failed outcome and the
    /// request hot path serves LKG / 500.
    #[test]
    fn in_flight_waiter_sees_failure_outcome() {
        use std::thread;
        let r = PageRegistry::new();
        let p = page("/", HttpMethod::Get);
        r.register(p.clone(), modid("routes/index.tsp"));
        // First commit so LKG is established.
        let g1 = r.begin_build(&p).unwrap();
        g1.commit(vec![], "lkg-body".to_string());
        r.mark_dirty(&p);
        // Owner thread fails the build.
        let r_a = r.clone();
        let p_a = p.clone();
        let a = thread::spawn(move || {
            let guard = r_a.begin_build(&p_a).expect("begin");
            let shared = r_a.join_in_flight(&p_a).expect("in-flight");
            thread::sleep(std::time::Duration::from_millis(20));
            guard.fail("boom".to_string());
            shared
        });
        // Waiter: sees Failed.
        let r_b = r.clone();
        let p_b = p.clone();
        let b = thread::spawn(move || {
            for _ in 0..100 {
                if matches!(
                    r_b.snapshot(&p_b).map(|s| s.state),
                    Some(PageState::Building)
                ) {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(2));
            }
            let shared = r_b.join_in_flight(&p_b).expect("in-flight");
            let guard = shared.state.lock().expect("lock");
            shared.wait(guard)
        });
        let outcome = b.join().expect("B");
        let _ = a.join().expect("A");
        match outcome {
            InFlightState::Done(BuildOutcome::Failed(msg)) => assert_eq!(msg, "boom"),
            other => panic!("expected Failed(boom), got {other:?}"),
        }
        // LKG is intact.
        let lkg = r.read_lkg_arc(&p).unwrap();
        assert_eq!(*lkg, "lkg-body");
    }

    #[test]
    fn unregister_path_drops_all_method_slots() {
        use crate::router::{HttpMethod, Route};
        let r = PageRegistry::new();
        let route = Route {
            path: "/x".to_string(),
            source: PathBuf::from("routes/x.tsp"),
            methods: vec![HttpMethod::Get, HttpMethod::Post],
        };
        r.register_route(&route);
        assert_eq!(r.all_page_refs().len(), 2);
        let removed = r.unregister_path("/x");
        assert_eq!(removed, 2);
        assert_eq!(r.all_page_refs().len(), 0);
        // Idempotent: removing again is a no-op.
        assert_eq!(r.unregister_path("/x"), 0);
    }

    #[test]
    fn unregister_one_page_ref_does_not_drop_siblings() {
        use crate::router::{HttpMethod, Route};
        let r = PageRegistry::new();
        let route = Route {
            path: "/x".to_string(),
            source: PathBuf::from("routes/x.tsp"),
            methods: vec![HttpMethod::Get, HttpMethod::Post],
        };
        r.register_route(&route);
        let get_ref = PageRef { route: "/x".to_string(), method: HttpMethod::Get };
        let post_ref = PageRef { route: "/x".to_string(), method: HttpMethod::Post };
        assert!(r.unregister(&get_ref));
        // The Post slot survives.
        assert_eq!(r.all_page_refs().len(), 1);
        assert!(r.unregister(&get_ref) == false);
        assert!(r.unregister(&post_ref));
        assert_eq!(r.all_page_refs().len(), 0);
    }
}