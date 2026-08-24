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
//! event-loop + FD lifecycle that the side-by-side v2 host does
//! not have wired yet. Polling with a real-time source-hash diff
//! is the smallest implementation that satisfies the lazy-reload
//! contract; swapping the backend later (watcher.rs is a single
//! module) is a localized change.
//!
//! The poll loop:
//! ```text
//! every poll_ms:
//!   snapshot = read all source files under routes_root (incl.
//!              .ts/.tsx deps)
//!   for each file:
//!     new_hash = SourceHash::compute(text)
//!     if new_hash != last_seen[path]  -> changed
//!   if changed:
//!     graph.affected_pages(changed_id)  -> PageRefs
//!     for each affected PageRef: registry.mark_dirty(page)
//!   update last_seen
//! ```
//!
//! The `watcher` thread owns the `ModuleGraph`'s reverse edges and
//! writes ONLY via `PageRegistry::mark_dirty` -- it never touches
//! `current` / `last_known_good` generation pointers. The request
//! threads and the watcher thread therefore never contend on the
//! same lock: the registry's `Mutex` is the single serialisation
//! point and each side takes it for a few instructions.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::generation::{MarkDirtyResult, PageRegistry};
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
#[derive(Debug)]
pub struct WatchConfig {
    /// Directory to watch (routes/ root + its recursive subdirs).
    pub routes_root: PathBuf,
    /// Poll interval in milliseconds.
    pub poll_ms: u64,
}

/// Shared handle for the host to stop the watcher thread on
/// shutdown.
#[derive(Debug, Clone)]
pub struct WatcherHandle {
    stop: Arc<AtomicBool>,
    thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl WatcherHandle {
    /// True while the watcher thread is running.
    pub fn is_running(&self) -> bool {
        !self.stop.load(Ordering::Acquire)
    }

    /// Signal the poll loop to stop on its next tick. The thread
    /// is NOT joined here (the host may be mid-shutdown); the
    /// `Drop` impl of the handle does the join if the thread is
    /// still alive.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
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
}

/// A poll cycle. Reads every source file under `root` (route +
/// .ts/.tsx deps), computes hashes, diffs against `last_seen`,
/// and marks affected slots dirty via the reverse edges of
/// `graph`.
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
) -> Result<(usize, usize), String> {
    let desired = match RouteTable::scan(root) {
        Ok(t) => t,
        Err(e) => return Err(format!("scan failed: {e}")),
    };
    let desired_paths: std::collections::HashSet<String> =
        desired.paths().into_iter().collect();
    let actual_paths: std::collections::HashSet<String> =
        table.paths().into_iter().collect();

    let added: Vec<String> = desired_paths.difference(&actual_paths).cloned().collect();
    let removed: Vec<String> = actual_paths.difference(&desired_paths).cloned().collect();

    // Apply additions first. `register_route` runs
    // `page::prepare` (file I/O); if a file disappears
    // between the scan and the register we ignore the
    // result (the next poll will re-resolve it).
    for path in &added {
        let route = match desired.get_by_path(path) {
            Some(r) => r.clone(),
            None => continue,
        };
        if let Err(e) = table.add(route.clone()) {
            // Duplicate path: race with another watcher
            // tick, or the same path was added between the
            // scan and now. Log + continue.
            eprintln!(
                "TSPv2PoC1: watch: skipped add {}: {e}",
                route.source.display()
            );
            continue;
        }
        registry.register_route(&route);
        // Mark the freshly-registered slots dirty so the
        // next request triggers a build (the slot starts
        // in Unloaded state which already triggers build
        // on first request, but mark_dirty is explicit and
        // keeps the watcher the single owner of dirty
        // state).
        for &method in &route.methods {
            let page_ref = crate::generation::PageRef {
                route: route.path.clone(),
                method,
            };
            let _ = registry.mark_dirty(&page_ref);
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
        eprintln!(
            "TSPv2PoC1: watch: removed route {path} (dropped {n} slot(s))"
        );
    }

    Ok((added.len(), removed.len()))
}

pub fn poll_once(
    root: &Path,
    _graph: &crate::module_graph::ModuleGraph,
    table: &RouteTable,
    registry: &PageRegistry,
    last_seen: &mut HashMap<PathBuf, SourceHash>,
) -> PollStats {
    let mut stats = PollStats::default();
    let mut current: HashMap<PathBuf, SourceHash> = HashMap::new();

    collect_sources(root, &mut current, &mut stats);

    // Find files whose hash changed (new file or content change).
    for (path, hash) in &current {
        if last_seen.get(path) != Some(hash) {
            stats.changed_files.push(path.clone());
        }
    }
    // A file that disappeared is also a change (the request will
    // get a prepare error and report 500 / revert to LKG).
    for path in last_seen.keys() {
        if !current.contains_key(path) {
            stats.changed_files.push(path.clone());
        }
    }

    // Reconcile the live RouteTable + PageRegistry with the
    // freshly-scanned view of the routes dir. A failure
    // here (e.g. an unsupported file shape) leaves the
    // table + registry untouched and surfaces the error
    // in PollStats so the watcher's outer loop can log it.
    match reconcile_routes(root, table, registry) {
        Ok((added, removed)) => {
            stats.routes_added = added;
            stats.routes_removed = removed;
        }
        Err(msg) => {
            stats.reconcile_error = Some(msg);
        }
    }

    // For each changed file, mark the affected page slots dirty.
    //
    // Slice 11's granularity is "any change dirties every page
    // under the routes root". This is conservative (a change to
    // `lib/util.ts` dirties pages that don't import it, and they
    // rebuild unnecessarily) but SIMPLE and correct: the next
    // request always serves the newest version, and the rebuild
    // cost of a small app is negligible. Slice 12 replaces this
    // with the precise source->PageRef index (via
    // `ModuleGraph.importers_of` + a registry reverse map) so
    // only truly-affected pages rebuild.
    if !stats.changed_files.is_empty() {
        for page_ref in registry.all_page_refs() {
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
    last_seen.retain(|k, _| current.contains_key(k));
    for (path, hash) in current {
        last_seen.insert(path, hash);
    }

    stats
}

/// Recursively collect all source files under `root` into
/// `current`, computing their content hash. Non-source files
/// (`.git`, node_modules, binary) are skipped.
fn collect_sources(
    root: &Path,
    current: &mut HashMap<PathBuf, SourceHash>,
    stats: &mut PollStats,
) {
    let Ok(entries) = fs::read_dir(root) else {
        // Root gone -> nothing to watch (the poll loop retries).
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_sources(&path, current, stats);
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
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        if let Ok(text) = String::from_utf8(bytes) {
            let hash = SourceHash::compute(&text);
            current.insert(path.clone(), hash);
        }
    }
}


/// Spawn the watcher thread. Returns a handle the host can use
/// to stop it (and that joins on drop).
///
/// The thread owns `last_seen`; the `ModuleGraph` and
/// `PageRegistry` are ARC'd into the closure. The graph is
/// frozen at spawn (slice 12 will let the watcher pick up new
/// routes without a restart).
pub fn spawn(
    config: WatchConfig,
    graph: Arc<crate::module_graph::ModuleGraph>,
    table: Arc<RouteTable>,
    registry: Arc<PageRegistry>,
) -> WatcherHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_in_thread = stop.clone();
    let thread = thread::Builder::new()
        .name("tsp-v2-watcher".to_string())
        .spawn(move || {
            let poll = Duration::from_millis(config.poll_ms);
            // Initial snapshot so the first poll only reports
            // changes AFTER we start (not the whole tree).
            let mut last_seen: HashMap<PathBuf, SourceHash> = HashMap::new();
            collect_sources(&config.routes_root, &mut last_seen, &mut PollStats::default());

            while !stop_in_thread.load(Ordering::Acquire) {
                thread::sleep(poll);
                if stop_in_thread.load(Ordering::Acquire) {
                    break;
                }
                let changed = poll_once(
                    &config.routes_root,
                    &graph,
                    &table,
                    &registry,
                    &mut last_seen,
                );
                if !changed.changed_files.is_empty() {
                    eprintln!(
                        "TSPv2PoC1: watch: {} file(s) changed, {} page(s) marked dirty",
                        changed.changed_files.len(),
                        changed.marked
                    );
                }
            }
        })
        .expect("spawn watcher thread");

    WatcherHandle {
        stop,
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
        let s1 = poll_once(&dir, &ModuleGraph::new(), &RouteTable::empty(), &registry, &mut last_seen);
        assert!(s1.changed_files.is_empty(), "unchanged poll should report nothing, got {:?}", s1.changed_files);

        // Second poll (unchanged): still no changes.
        let s2 = poll_once(&dir, &ModuleGraph::new(), &RouteTable::empty(), &registry, &mut last_seen);
        assert!(s2.changed_files.is_empty());

        // Change the file.
        fs::write(&idx, "<h1>two</h1>").unwrap();
        let s3 = poll_once(&dir, &ModuleGraph::new(), &RouteTable::empty(), &registry, &mut last_seen);
        assert_eq!(s3.changed_files, vec![idx.clone()], "changed file should be detected");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn poll_detects_new_file() {
        let dir = temp_dir();
        let mut last_seen = HashMap::new();
        let registry = PageRegistry::new();
        poll_once(&dir, &ModuleGraph::new(), &RouteTable::empty(), &registry, &mut last_seen);

        let new_file = dir.join("new.tsp");
        fs::write(&new_file, "<p>hello</p>").unwrap();
        let s = poll_once(&dir, &ModuleGraph::new(), &RouteTable::empty(), &registry, &mut last_seen);
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
        poll_once(&dir, &ModuleGraph::new(), &RouteTable::empty(), &registry, &mut last_seen);

        fs::remove_file(&idx).unwrap();
        let s = poll_once(&dir, &ModuleGraph::new(), &RouteTable::empty(), &registry, &mut last_seen);
        assert!(s.changed_files.contains(&idx), "deleted file should be reported as changed");

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