//! Filesystem watcher for TSP v2 slice 11 (plan sect.22).
//!
//! See `tsp-v2-plan.md` sect.22.1-22.3:
//! - 22.1: watcher produces `Changed(path)` events.
//! - 22.2: **lazy reload** -- a file change marks the affected
//!   `PageSlot` dirty; the rebuild happens on the NEXT request
//!   (not immediately).
//! - 22.3: eager reload is a future option (`[dev] reload = "eager"`).
//!
//! Implementation: a polling backend (checks file mtime + content
//! hash every `poll_ms`). This is deliberately NOT the
//! `bun_watcher` crate integration from slice 7's spike -- bun's
//! watcher is platform-native (inotify / ReadDirectoryChangesW)
//! and its `WatcherContext` callback API requires the full Bun
//! event-loop + FD lifecycle that the native host does not have wired yet.
//! Polling with a real-time source-hash diff
//! is the smallest implementation that satisfies the lazy-reload
//! contract; swapping the backend later (watcher.rs is a single
//! module) is a localized change.
//!
//! The polling fallback loop:
//! ```text
//! every poll_ms:
//!   snapshot = read all source files under routes_root (incl.
//!              .ts/.tsx deps)
//!   for each file:
//!     new_hash = SourceHash::compute(text)
//!     if new_hash != last_seen[path]  -> changed
//!   if changed:
//!     dependency_index.affected_pages(changed) -> PageRefs
//!     for each affected PageRef: registry.mark_dirty(page)
//!   update last_seen
//! ```
//!
//! The watcher writes only through route reconciliation and
//! `PageRegistry::mark_dirty`; it never touches `current` /
//! `last_known_good` generation pointers. The request threads and the
//! watcher thread therefore share only the short-lived route/registry
//! locks, never a build generation itself.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::generation::{MarkDirtyResult, PageRef, PageRegistry};
use crate::invalidation_bus::InvalidationBus;
use crate::metrics;
use crate::module_graph::SourceHash;
use crate::router::RouteTable;

/// Default poll interval. 500ms is long enough that editor atomic
/// saves (write-tmp-then-rename) settle before we read, and short
/// enough that a dev edit feels instant on the next request.
///
/// Not currently read by `spawn` (the config carries its own
/// `poll_ms`); used by the host (`bin/tspserver_v2.rs`) when it
/// constructs a `WatchConfig` from the CLI / defaults.
pub const DEFAULT_POLL_MS: u64 = 500;

/// The watcher thread's view of the routes root. It owns nothing
/// mutable; it reads files and writes `mark_dirty` results.
///
/// `Debug` is implemented manually (no `#[derive(Debug)]`) because
/// the `on_config_reload` callback is a `dyn Fn` which does not
/// implement `Debug`; the manual impl omits the callback from
/// the printed shape.
pub struct WatchConfig {
    /// Directory to watch (routes/ root + its recursive subdirs).
    pub routes_root: PathBuf,
    /// Poll interval in milliseconds.
    pub poll_ms: u64,
    /// §22.3: optional config file to watch for hot reload.
    /// When the file's content hash changes, the watcher
    /// calls `on_config_reload` with the new text. The
    /// callback parses + applies the snapshot; the
    /// watcher just logs the result. A `None` value
    /// disables config watching (e.g. when the host
    /// has no config-driven services).
    pub config_path: Option<PathBuf>,
    /// §22.3: callback invoked with the new config text
    /// when the watched config file's content hash
    /// changes. The callback returns a human-readable
    /// summary (e.g. `"applied 3 services: hits, kv,
    /// flags"`) on success, or an error message on
    /// failure. The watcher logs both to stderr; the
    /// callback is responsible for any state mutation
    /// (typically `ServiceRegistry::apply_config_snapshot`
    /// under a write lock).
    pub on_config_reload: Option<Arc<dyn Fn(&str) -> Result<String, String> + Send + Sync>>,
}

impl std::fmt::Debug for WatchConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WatchConfig")
            .field("routes_root", &self.routes_root)
            .field("poll_ms", &self.poll_ms)
            .field("config_path", &self.config_path)
            .field(
                "on_config_reload",
                &self.on_config_reload.as_ref().map(|_| "<callback>"),
            )
            .finish()
    }
}

/// Shared handle for the host to stop the watcher thread on
/// shutdown.
#[derive(Debug, Clone)]
pub struct WatcherHandle {
    stop: Arc<AtomicBool>,
    wake: Arc<(Mutex<()>, Condvar)>,
    thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl WatcherHandle {
    /// True while the watcher thread is running.
    pub fn is_running(&self) -> bool {
        if self.stop.load(Ordering::Acquire) {
            return false;
        }
        self.thread
            .lock()
            .map(|guard| guard.as_ref().is_some_and(|thread| !thread.is_finished()))
            .unwrap_or(false)
    }

    /// Signal the poll loop to stop immediately. The thread is
    /// NOT joined here (the host may be mid-shutdown); the
    /// `Drop` impl of the handle does the join if the thread is
    /// still alive.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        self.wake.1.notify_one();
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.wake.1.notify_one();
        if let Ok(mut guard) = self.thread.lock() {
            if let Some(handle) = guard.take() {
                // Poll delay is <= poll_ms; join should return
                // quickly unless the poll loop is blocked on a
                // slow FS.
                let _ = handle.join();
            }
        }
    }
}

/// Result of one poll cycle: which files changed (by canonical
/// path) and how many of the affected page slots got marked
/// dirty. Used by tests and by the dev-inspector future slice.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PollStats {
    /// File paths whose source hash changed since the last poll.
    pub changed_files: Vec<PathBuf>,
    /// Slots we marked dirty, grouped by MarkDirtyResult kind.
    pub marked: usize,
    pub already_dirty: usize,
    pub build_in_flight: usize,
    pub unknown_page: usize,
    /// Routes added to `RouteTable` + `PageRegistry` this poll
    /// (slice 15a: spec sect.12 route file creation).
    pub routes_added: usize,
    /// Routes removed from `RouteTable` + `PageRegistry` this
    /// poll (slice 15a: spec sect.33.5 deleted route).
    pub routes_removed: usize,
    /// A `reconcile_routes` error -- the watcher prints it
    /// and leaves the table unchanged. The next poll will
    /// retry the scan.
    pub reconcile_error: Option<String>,
    /// False when one or more files could not be read consistently. A
    /// partial snapshot must not be interpreted as a batch of deletions.
    pub snapshot_complete: bool,
    pub snapshot_errors: usize,
}

/// Metadata and content identity for one watched source file. The hash is
/// reused when size and mtime are unchanged, avoiding a full read on every
/// polling tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSnapshot {
    pub modified: Option<SystemTime>,
    pub len: u64,
    pub hash: SourceHash,
}

/// Reverse dependency index used by the watcher. It is deliberately kept
/// outside `ModuleGraph` for now because the v2 build pipeline does not yet
/// return its resolved dependency list. The index is conservative: when a
/// source cannot be resolved or read, `complete` becomes false and the
/// watcher falls back to invalidating every registered page.
#[derive(Debug, Default, Clone)]
struct DependencyIndex {
    pages_by_file: HashMap<PathBuf, HashSet<PageRef>>,
    complete: bool,
}

impl DependencyIndex {
    fn rebuild(root: &Path, table: &RouteTable, registry: &PageRegistry) -> Self {
        let canonical_root = canonical_watch_path(root);
        let mut index = Self {
            pages_by_file: HashMap::new(),
            complete: true,
        };
        let routes: HashMap<String, crate::router::Route> = table
            .iter()
            .into_iter()
            .map(|route| (route.path.clone(), route))
            .collect();

        for page in registry.all_page_refs() {
            let Some(route) = routes.get(&page.route) else {
                index.complete = false;
                continue;
            };
            let mut files = HashSet::new();
            let mut visiting = HashSet::new();
            if collect_dependency_files(&canonical_root, &route.source, &mut files, &mut visiting)
                .is_err()
            {
                index.complete = false;
                // Keep the root source in the index even when an import is
                // temporarily unreadable. A route edit must still reload
                // that page while the index is in fallback mode.
                files.insert(canonical_watch_path(&route.source));
            }
            for file in files {
                index
                    .pages_by_file
                    .entry(file)
                    .or_default()
                    .insert(page.clone());
            }
        }
        index
    }

    fn affected_pages(&self, changed_files: &[PathBuf]) -> Option<HashSet<PageRef>> {
        if !self.complete {
            return None;
        }
        let mut pages = HashSet::new();
        for path in changed_files {
            if let Some(affected) = self.pages_by_file.get(&canonical_watch_path(path)) {
                pages.extend(affected.iter().cloned());
            }
        }
        // A route source must always map to its own page. If filesystem
        // canonicalization or a transient route-table race leaves the index
        // without that mapping, invalidate every page rather than silently
        // serving stale output.
        if pages.is_empty() && !changed_files.is_empty() {
            None
        } else {
            Some(pages)
        }
    }
}

fn canonical_watch_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn collect_dependency_files(
    root: &Path,
    source: &Path,
    files: &mut HashSet<PathBuf>,
    visiting: &mut HashSet<PathBuf>,
) -> Result<(), ()> {
    let canonical = canonical_watch_path(source);
    if !visiting.insert(canonical.clone()) {
        return Ok(());
    }
    if !canonical.starts_with(root) {
        return Err(());
    }
    files.insert(canonical.clone());
    let text = fs::read_to_string(&canonical).map_err(|_| ())?;
    for specifier in crate::module_graph::extract_imports(&text) {
        let Some(imported) = resolve_local_import(&canonical, specifier.as_path()) else {
            // Bare specifiers belong to the package/runtime resolver and are
            // intentionally outside the routes-root watcher index.
            continue;
        };
        collect_dependency_files(root, &imported, files, visiting)?;
    }
    Ok(())
}

fn resolve_local_import(importer: &Path, specifier: &Path) -> Option<PathBuf> {
    let specifier_text = specifier.to_string_lossy();
    if !specifier.is_absolute() && !specifier_text.starts_with('.') {
        return None;
    }
    let base = if specifier.is_absolute() {
        specifier.to_path_buf()
    } else {
        importer.parent()?.join(specifier)
    };
    let mut candidates = vec![base.clone()];
    if base.extension().is_none() {
        for extension in ["ts", "tsx", "js", "jsx"] {
            candidates.push(base.with_extension(extension));
        }
        for extension in ["ts", "tsx", "js", "jsx"] {
            candidates.push(base.join(format!("index.{extension}")));
        }
    }
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| canonical_watch_path(&candidate))
}

/// A poll cycle. Reads every source file under `root` (route +
/// local dependency files), computes hashes, diffs against
/// `last_seen`, and marks only pages affected by the changed
/// files dirty via the watcher-owned dependency index.
///
/// `last_seen` is threaded through the call because the loop owns
/// it; it is not stored in the struct so the watcher stays
/// stateless and tests can feed a fresh `last_seen` on each call.
/// Reconcile the live `RouteTable` and `PageRegistry` with a
/// freshly-scanned view of the routes dir. Returns (added,
/// removed) counts for the `PollStats`. New files in the
/// routes dir produce a fresh `Route` and one slot per
/// method; removed files drop both the table entry and the
/// registry slots.
///
/// The `RouteTable::scan` call may fail (a `.tsp` file with
/// an unsupported shape, or a permission error on a
/// subdirectory). In that case we return the error and
/// leave the existing table + registry untouched -- a
/// half-applied reconciliation would be worse than
/// rejecting the whole tick.
fn reconcile_routes(
    root: &Path,
    table: &RouteTable,
    registry: &PageRegistry,
    changed_files: &[PathBuf],
) -> Result<(usize, usize), String> {
    let desired = match RouteTable::scan(root) {
        Ok(t) => t,
        Err(e) => return Err(format!("scan failed: {e}")),
    };
    let desired_routes = desired.iter();
    let actual_routes = table.iter();
    let desired_paths: HashSet<String> = desired_routes.iter().map(|r| r.path.clone()).collect();
    let actual_paths: HashSet<String> = actual_routes.iter().map(|r| r.path.clone()).collect();

    let added: Vec<String> = desired_paths.difference(&actual_paths).cloned().collect();
    let removed: Vec<String> = actual_paths.difference(&desired_paths).cloned().collect();

    let changed: HashSet<PathBuf> = changed_files.iter().cloned().collect();

    // A route's methods are source-derived metadata. RouteTable::scan only
    // knows the path shape, so prepare changed/new route files before they
    // enter the live table. This also makes a same-path source replacement
    // observable to the registry.
    let mut replacements: Vec<(crate::router::Route, crate::router::Route)> = Vec::new();
    for desired_route in desired_routes {
        let Some(old) = actual_routes.iter().find(|r| r.path == desired_route.path) else {
            continue;
        };
        if old.source != desired_route.source || changed.contains(&desired_route.source) {
            let normalized = prepare_route(desired_route)?;
            if old.source != normalized.source || old.methods != normalized.methods {
                replacements.push((old.clone(), normalized));
            }
        }
    }

    // Apply additions first. Register the slots before publishing the route
    // so a request cannot observe a route with no PageRegistry entries.
    for path in &added {
        let route = match desired.get_by_path(path) {
            Some(r) => r.clone(),
            None => continue,
        };
        let route = prepare_route(route)?;
        registry.register_route(&route);
        for &method in &route.methods {
            let page_ref = crate::generation::PageRef {
                route: route.path.clone(),
                method,
            };
            let _ = registry.mark_dirty(&page_ref);
        }
        if let Err(e) = table.add(route.clone()) {
            registry.unregister_path(&route.path);
            return Err(format!("add route {} failed: {e}", route.path));
        }
    }

    // Replace same-path routes after their new slots/source identities are
    // ready. Removed method slots are dropped only after the table points at
    // the new method set.
    for (old, route) in replacements {
        registry.register_route(&route);
        let source = crate::module_graph::ModuleId::from_path(&route.source);
        for &method in &route.methods {
            let page_ref = crate::generation::PageRef {
                route: route.path.clone(),
                method,
            };
            registry.update_source(&page_ref, source.clone());
            let _ = registry.mark_dirty(&page_ref);
        }
        if !table.replace_by_path(route.clone()) {
            return Err(format!(
                "replace route {} failed: route disappeared",
                route.path
            ));
        }
        for method in old.methods {
            if !route.methods.contains(&method) {
                registry.unregister(&crate::generation::PageRef {
                    route: route.path.clone(),
                    method,
                });
            }
        }
    }

    // Apply removals. `unregister_path` drops every slot
    // with the matching route field; in-flight requests
    // already pinned to those slots continue to serve
    // from the LKG / current generation they pinned
    // (the slot's data is dropped but the pinned
    // `Arc<String>` is still held by the request).
    for path in &removed {
        table.remove_by_path(path);
        let n = registry.unregister_path(path);
        eprintln!("TSPv2PoC1: watch: removed route {path} (dropped {n} slot(s))");
    }

    Ok((added.len(), removed.len()))
}

fn prepare_route(mut route: crate::router::Route) -> Result<crate::router::Route, String> {
    let prepared = crate::page::prepare(&route)
        .map_err(|e| format!("prepare {} failed: {e}", route.source.display()))?;
    route.methods = prepared.methods;
    Ok(route)
}

pub fn poll_once(
    root: &Path,
    _graph: &crate::module_graph::ModuleGraph,
    table: &RouteTable,
    registry: &PageRegistry,
    last_seen: &mut HashMap<PathBuf, FileSnapshot>,
) -> PollStats {
    let mut dependency_index = DependencyIndex::rebuild(root, table, registry);
    poll_once_with_index(root, table, registry, last_seen, &mut dependency_index)
}

fn poll_once_with_index(
    root: &Path,
    table: &RouteTable,
    registry: &PageRegistry,
    last_seen: &mut HashMap<PathBuf, FileSnapshot>,
    dependency_index: &mut DependencyIndex,
) -> PollStats {
    let mut stats = PollStats::default();
    let mut current: HashMap<PathBuf, FileSnapshot> = HashMap::new();

    stats.snapshot_complete =
        collect_sources_with_previous(root, &mut current, last_seen, &mut stats);

    // Find files whose hash changed (new file or content change).
    for (path, snapshot) in &current {
        if last_seen.get(path).map(|previous| previous.hash) != Some(snapshot.hash) {
            stats.changed_files.push(path.clone());
        }
    }
    // A file that disappeared is also a change (the removed route is
    // reconciled below). Only a complete snapshot can
    // prove that a file disappeared; an unreadable file is not a deletion.
    if stats.snapshot_complete {
        for path in last_seen.keys() {
            if !current.contains_key(path) {
                stats.changed_files.push(path.clone());
            }
        }
    }

    // Reconcile the live RouteTable + PageRegistry with the
    // freshly-scanned view of the routes dir. A failure
    // here (e.g. an unsupported file shape) leaves the
    // table + registry untouched and surfaces the error
    // in PollStats so the watcher's outer loop can log it.
    if stats.snapshot_complete {
        match reconcile_routes(root, table, registry, &stats.changed_files) {
            Ok((added, removed)) => {
                stats.routes_added = added;
                stats.routes_removed = removed;
            }
            Err(msg) => {
                stats.reconcile_error = Some(msg);
            }
        }
    }

    // Rebuild the conservative reverse index after a change. This picks up
    // newly-created dependencies and import-list edits before choosing the
    // affected pages. If indexing is incomplete, fall back to all pages so a
    // missing dependency can never silently suppress a reload.
    if !stats.changed_files.is_empty() {
        metrics::global().record_reload();
        *dependency_index = DependencyIndex::rebuild(root, table, registry);
        let affected = dependency_index
            .affected_pages(&stats.changed_files)
            .unwrap_or_else(|| registry.all_page_refs().into_iter().collect());
        for page_ref in affected {
            match registry.mark_dirty(&page_ref) {
                MarkDirtyResult::Marked => stats.marked += 1,
                MarkDirtyResult::AlreadyDirty => stats.already_dirty += 1,
                MarkDirtyResult::BuildInFlight => stats.build_in_flight += 1,
                MarkDirtyResult::AlreadyFirstLoad => {}
                MarkDirtyResult::UnknownPage => stats.unknown_page += 1,
            }
        }
    }

    // Update last_seen to the current snapshot. Files that
    // disappeared are dropped (their entry is removed by the
    // retain below).
    if stats.snapshot_complete {
        last_seen.retain(|k, _| current.contains_key(k));
    }
    for (path, snapshot) in current {
        last_seen.insert(path, snapshot);
    }

    stats
}

/// §22.3 config-file poll helper. Reads the file at
/// `path` and computes its content hash; if the hash
/// differs from `last_hash`, returns `Ok(Some(text))`
/// (the new content) and updates `last_hash`. A missing
/// file is treated as "no change" (the host's
/// `TSP_CONFIG` env may legitimately point at a
/// non-existent path that the operator later creates;
/// the first poll that sees the file fires the
/// callback). A read error returns `Err(...)` so the
/// watcher can log it but retain the previous state.
///
/// The caller is responsible for invoking the
/// `on_config_reload` callback when this function
/// returns `Some(text)`.
fn poll_config_once(
    path: &Path,
    last_hash: &mut Option<SourceHash>,
) -> Result<Option<String>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        // ENOENT is not an error here -- the operator
        // may add a config file later. Any other error
        // (permission denied, etc.) is reported to the
        // caller for logging; the registry's state is
        // preserved.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    let new_hash = SourceHash::compute(&text);
    if last_hash.as_ref() == Some(&new_hash) {
        return Ok(None);
    }
    *last_hash = Some(new_hash);
    Ok(Some(text))
}

/// Recursively collect all source files under `root` into
/// `current`, computing their content hash. Non-source files
/// (`.git`, node_modules, binary) are skipped.
fn collect_sources(
    root: &Path,
    current: &mut HashMap<PathBuf, FileSnapshot>,
    stats: &mut PollStats,
) -> bool {
    collect_sources_with_previous(root, current, &HashMap::new(), stats)
}

fn collect_sources_with_previous(
    root: &Path,
    current: &mut HashMap<PathBuf, FileSnapshot>,
    previous: &HashMap<PathBuf, FileSnapshot>,
    stats: &mut PollStats,
) -> bool {
    let Ok(entries) = fs::read_dir(root) else {
        // Root gone or temporarily inaccessible. Do not convert the whole
        // tree into deletions; retain the previous snapshot and retry.
        stats.snapshot_errors += 1;
        return false;
    };
    let mut complete = true;
    for entry_result in entries {
        let Ok(entry) = entry_result else {
            stats.snapshot_errors += 1;
            complete = false;
            continue;
        };
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            stats.snapshot_errors += 1;
            complete = false;
            continue;
        };
        if file_type.is_dir() {
            if matches!(
                path.file_name().and_then(|n| n.to_str()),
                Some(".git" | "node_modules" | "target" | ".cache")
            ) {
                continue;
            }
            if !collect_sources_with_previous(&path, current, previous, stats) {
                complete = false;
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !matches!(ext, "tsp" | "ts" | "tsx" | "js" | "jsx") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            stats.snapshot_errors += 1;
            complete = false;
            continue;
        };
        let modified = metadata.modified().ok();
        if let Some(snapshot) = previous.get(&path) {
            if snapshot.len == metadata.len() && snapshot.modified == modified {
                current.insert(path.clone(), *snapshot);
                continue;
            }
        }
        let Ok(bytes) = fs::read(&path) else {
            stats.snapshot_errors += 1;
            complete = false;
            continue;
        };
        if let Ok(text) = String::from_utf8(bytes) {
            current.insert(
                path.clone(),
                FileSnapshot {
                    modified,
                    len: metadata.len(),
                    hash: SourceHash::compute(&text),
                },
            );
        } else {
            stats.snapshot_errors += 1;
            complete = false;
        }
    }
    complete
}

/// Spawn the watcher thread. Returns a handle the host can use
/// to stop it (and that joins on drop).
///
/// The thread owns `last_seen` and its dependency index. The graph argument
/// remains accepted for compatibility with the v2 host, but invalidation is
/// driven by the watcher-owned index so route and import changes can rebuild
/// it without restarting the process.
pub fn spawn(
    config: WatchConfig,
    graph: Arc<crate::module_graph::ModuleGraph>,
    table: Arc<RouteTable>,
    registry: Arc<PageRegistry>,
) -> WatcherHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_in_thread = stop.clone();
    let wake = Arc::new((Mutex::new(()), Condvar::new()));
    let wake_in_thread = wake.clone();
    let thread = thread::Builder::new()
        .name("tsp-v2-watcher".to_string())
        .spawn(move || {
            let poll = Duration::from_millis(config.poll_ms);
            // Initial snapshot so the first poll only reports
            // changes AFTER we start (not the whole tree).
            let mut last_seen: HashMap<PathBuf, FileSnapshot> = HashMap::new();
            collect_sources(&config.routes_root, &mut last_seen, &mut PollStats::default());
            let mut dependency_index = DependencyIndex::rebuild(
                &config.routes_root,
                &table,
                &registry,
            );
            let mut invalidation_bus = std::env::var_os("TSP_INVALIDATION_FILE")
                .map(PathBuf::from)
                .and_then(|path| match InvalidationBus::open(path) {
                    Ok(bus) => Some(bus),
                    Err(error) => {
                        eprintln!("TSPv2PoC1: invalidation bus disabled: {error}");
                        None
                    }
                });
            // Kept in the spawn API for compatibility with the v2 host while
            // the watcher-owned index is being migrated away from the frozen
            // graph representation.
            let _graph = graph;

            // §22.3: per-config-file state. `last_config_hash` is
            // `None` until the first poll sees a file (or stays
            // `None` forever if the host has no config-driven
            // services). On every tick we re-stat + re-read the
            // file; a content-hash change fires the callback.
            let mut last_config_hash: Option<SourceHash> = None;
            // Initial sync so the first poll only reports
            // CHANGES, not the boot-time state. If the file
            // exists at boot, the host already loaded it via
            // `load_config_services`; we just
            // record its current hash so a no-op edit does
            // not fire a reload.
            if let Some(path) = &config.config_path {
                if let Ok(text) = std::fs::read_to_string(path) {
                    last_config_hash = Some(SourceHash::compute(&text));
                }
            }

            while !stop_in_thread.load(Ordering::Acquire) {
                let guard = wake_in_thread.0.lock().expect("watcher wake lock poisoned");
                let _ = wake_in_thread
                    .1
                    .wait_timeout(guard, poll)
                    .expect("watcher wake condvar poisoned");
                if stop_in_thread.load(Ordering::Acquire) {
                    break;
                }
                let changed = poll_once_with_index(
                    &config.routes_root,
                    &table,
                    &registry,
                    &mut last_seen,
                    &mut dependency_index,
                );
                if !changed.changed_files.is_empty() {
                    let generation = crate::jsc_bridge::bump_execution_generation();
                    eprintln!("TSPv2PoC1: published execution generation {generation}");
                }
                if let Some(bus) = invalidation_bus.as_mut() {
                    if let Err(error) = bus.publish(&changed.changed_files) {
                        eprintln!("TSPv2PoC1: invalidation publish failed: {error}");
                    }
                    match bus.read_since() {
                        Ok(remote_files) if !remote_files.is_empty() => {
                            let affected = dependency_index
                                .affected_pages(&remote_files)
                                .unwrap_or_else(|| registry.all_page_refs().into_iter().collect());
                            for page_ref in affected {
                                let _ = registry.mark_dirty(&page_ref);
                            }
                            eprintln!(
                                "TSPv2PoC1: invalidation bus received {} file(s)",
                                remote_files.len()
                            );
                        }
                        Ok(_) => {}
                        Err(error) => eprintln!("TSPv2PoC1: invalidation read failed: {error}"),
                    }
                }
                if !changed.changed_files.is_empty() {
                    eprintln!(
                        "TSPv2PoC1: watch: {} file(s) changed, {} page(s) marked dirty",
                        changed.changed_files.len(),
                        changed.marked
                    );
                }
                if let Some(error) = changed.reconcile_error {
                    eprintln!("TSPv2PoC1: watch: route reconciliation failed: {error}");
                }
                if !changed.snapshot_complete && changed.snapshot_errors > 0 {
                    eprintln!(
                        "TSPv2PoC1: watch: source snapshot incomplete ({} error(s)); retaining previous snapshot",
                        changed.snapshot_errors
                    );
                }

                // §22.3 config hot reload. A missing or
                // unparsable file is a NO-OP (the host's
                // boot-time read would have surfaced a
                // hard error; the watcher's only job is to
                // react to a SUCCESSFUL parse-then-apply
                // sequence, which the callback returns
                // summary text for).
                if let (Some(path), Some(callback)) =
                    (&config.config_path, &config.on_config_reload)
                {
                    match poll_config_once(path, &mut last_config_hash) {
                        Ok(Some(text)) => match callback(&text) {
                            Ok(summary) => eprintln!(
                                "TSPv2PoC1: config reloaded from {}: {}",
                                path.display(),
                                summary
                            ),
                            Err(error) => eprintln!(
                                "TSPv2PoC1: config reload apply failed: {error}"
                            ),
                        },
                        Ok(None) => {}
                        Err(error) => eprintln!(
                            "TSPv2PoC1: config reload read failed: {error} (retaining previous snapshot)"
                        ),
                    }
                }
            }
        })
        .expect("spawn watcher thread");

    WatcherHandle {
        stop,
        wake,
        thread: Arc::new(Mutex::new(Some(thread))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::PageRegistry;
    use crate::module_graph::ModuleGraph;
    use std::sync::Arc;

    fn temp_dir() -> PathBuf {
        let mut p = std::env::temp_dir();
        let suffix = format!(
            "tsp-v2-watcher-test-{}-{}.d",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        p.push(suffix);
        fs::create_dir_all(&p).expect("create temp dir");
        p
    }

    #[test]
    fn poll_detects_content_change() {
        let dir = temp_dir();
        let idx = dir.join("index.tsp");
        fs::write(&idx, "<h1>one</h1>").unwrap();

        let registry = PageRegistry::new();

        // Seed last_seen with the file's current hash so the
        // change detection is relative to a known baseline. The
        // watcher thread does this internally; the test does it
        // by hand so the first poll does NOT report the file as
        // changed just because it exists.
        let mut last_seen = HashMap::new();
        collect_sources(&dir, &mut last_seen, &mut PollStats::default());

        // First poll (unchanged): no changes.
        let s1 = poll_once(
            &dir,
            &ModuleGraph::new(),
            &RouteTable::empty(),
            &registry,
            &mut last_seen,
        );
        assert!(
            s1.changed_files.is_empty(),
            "unchanged poll should report nothing, got {:?}",
            s1.changed_files
        );

        // Second poll (unchanged): still no changes.
        let s2 = poll_once(
            &dir,
            &ModuleGraph::new(),
            &RouteTable::empty(),
            &registry,
            &mut last_seen,
        );
        assert!(s2.changed_files.is_empty());

        // Change the file.
        fs::write(&idx, "<h1>two</h1>").unwrap();
        let s3 = poll_once(
            &dir,
            &ModuleGraph::new(),
            &RouteTable::empty(),
            &registry,
            &mut last_seen,
        );
        assert_eq!(
            s3.changed_files,
            vec![idx.clone()],
            "changed file should be detected"
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn poll_detects_new_file() {
        let dir = temp_dir();
        let mut last_seen = HashMap::new();
        let registry = PageRegistry::new();
        poll_once(
            &dir,
            &ModuleGraph::new(),
            &RouteTable::empty(),
            &registry,
            &mut last_seen,
        );

        let new_file = dir.join("new.tsp");
        fs::write(&new_file, "<p>hello</p>").unwrap();
        let s = poll_once(
            &dir,
            &ModuleGraph::new(),
            &RouteTable::empty(),
            &registry,
            &mut last_seen,
        );
        assert!(s.changed_files.contains(&new_file));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn poll_detects_deleted_file() {
        let dir = temp_dir();
        let idx = dir.join("index.tsp");
        fs::write(&idx, "<h1>x</h1>").unwrap();
        let mut last_seen = HashMap::new();
        let registry = PageRegistry::new();
        poll_once(
            &dir,
            &ModuleGraph::new(),
            &RouteTable::empty(),
            &registry,
            &mut last_seen,
        );

        fs::remove_file(&idx).unwrap();
        let s = poll_once(
            &dir,
            &ModuleGraph::new(),
            &RouteTable::empty(),
            &registry,
            &mut last_seen,
        );
        assert!(
            s.changed_files.contains(&idx),
            "deleted file should be reported as changed"
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn poll_reconciles_changed_route_methods() {
        let dir = temp_dir();
        let route_file = dir.join("index.tsp");
        fs::write(&route_file, "export function GET() { return 'get'; }\n").unwrap();

        let table = RouteTable::scan(&dir).unwrap();
        let registry = PageRegistry::new();
        registry.register(
            crate::generation::PageRef {
                route: "/".to_string(),
                method: crate::router::HttpMethod::Get,
            },
            crate::module_graph::ModuleId::from_path(&route_file),
        );
        let mut last_seen = HashMap::new();
        collect_sources(&dir, &mut last_seen, &mut PollStats::default());

        fs::write(
            &route_file,
            "export function GET() { return 'get'; }\nexport function POST() { return 'post'; }\n",
        )
        .unwrap();
        let changed = poll_once(&dir, &ModuleGraph::new(), &table, &registry, &mut last_seen);
        assert_eq!(changed.routes_added, 0);
        assert_eq!(changed.routes_removed, 0);
        assert!(
            registry
                .snapshot(&crate::generation::PageRef {
                    route: "/".to_string(),
                    method: crate::router::HttpMethod::Post,
                })
                .is_some()
        );
        assert!(matches!(
            table.lookup("/", crate::router::HttpMethod::Post),
            crate::router::MatchResult::Found { .. }
        ));

        fs::write(&route_file, "export function GET() { return 'get'; }\n").unwrap();
        poll_once(&dir, &ModuleGraph::new(), &table, &registry, &mut last_seen);
        assert!(
            registry
                .snapshot(&crate::generation::PageRef {
                    route: "/".to_string(),
                    method: crate::router::HttpMethod::Post,
                })
                .is_none()
        );
        assert!(matches!(
            table.lookup("/", crate::router::HttpMethod::Post),
            crate::router::MatchResult::MethodNotAllowed { .. }
        ));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn incomplete_snapshot_does_not_report_deletions() {
        let dir = temp_dir();
        let file = dir.join("index.tsp");
        fs::write(&file, "export function GET() { return 'ok'; }\n").unwrap();
        let mut last_seen = HashMap::new();
        collect_sources(&dir, &mut last_seen, &mut PollStats::default());

        fs::remove_dir_all(&dir).unwrap();
        let stats = poll_once(
            &dir,
            &ModuleGraph::new(),
            &RouteTable::empty(),
            &PageRegistry::new(),
            &mut last_seen,
        );
        assert!(!stats.snapshot_complete);
        assert!(stats.changed_files.is_empty());
        assert!(last_seen.contains_key(&file));
    }

    #[test]
    fn poll_only_dirties_pages_that_import_changed_dependency() {
        let dir = temp_dir();
        let index_file = dir.join("index.tsp");
        let about_file = dir.join("about.tsp");
        let shared_file = dir.join("shared.ts");
        fs::write(
            &index_file,
            "import { value } from './shared.ts';\nexport function GET() { return value; }\n",
        )
        .unwrap();
        fs::write(&about_file, "export function GET() { return 'about'; }\n").unwrap();
        fs::write(&shared_file, "export const value = 'one';\n").unwrap();

        let table = RouteTable::scan(&dir).unwrap();
        let registry = PageRegistry::new();
        let index_page = crate::generation::PageRef {
            route: "/".to_string(),
            method: crate::router::HttpMethod::Get,
        };
        let about_page = crate::generation::PageRef {
            route: "/about".to_string(),
            method: crate::router::HttpMethod::Get,
        };
        registry.register(
            index_page.clone(),
            crate::module_graph::ModuleId::from_path(&index_file),
        );
        registry.register(
            about_page.clone(),
            crate::module_graph::ModuleId::from_path(&about_file),
        );
        for page in [&index_page, &about_page] {
            let guard = registry.begin_build(page).unwrap();
            guard.commit(vec![], "initial".to_string());
        }

        let mut last_seen = HashMap::new();
        collect_sources(&dir, &mut last_seen, &mut PollStats::default());
        fs::write(&shared_file, "export const value = 'two';\n").unwrap();

        let stats = poll_once(&dir, &ModuleGraph::new(), &table, &registry, &mut last_seen);
        assert_eq!(stats.marked, 1);
        assert_eq!(
            registry.snapshot(&index_page).unwrap().state,
            crate::generation::PageState::Dirty
        );
        assert_eq!(
            registry.snapshot(&about_page).unwrap().state,
            crate::generation::PageState::Clean
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn watcher_handle_stops_thread() {
        // Spawn with a long poll (10s) so we can observe the
        // thread is alive, then stop it.
        let dir = temp_dir();
        let graph = Arc::new(ModuleGraph::new());
        let registry = Arc::new(PageRegistry::new());
        let handle = spawn(
            WatchConfig {
                routes_root: dir.clone(),
                poll_ms: 100,
                // §22.3: tests that don't exercise config
                // hot reload leave both fields at their
                // default `None`. The watcher skips both
                // the routes poll AND the config poll
                // when nothing is set.
                config_path: None,
                on_config_reload: None,
            },
            graph,
            Arc::new(RouteTable::empty()),
            registry,
        );
        assert!(handle.is_running());
        handle.stop();
        // Drop joins the thread and should return quickly.
        drop(handle);
        fs::remove_dir_all(&dir).unwrap();
    }
}
