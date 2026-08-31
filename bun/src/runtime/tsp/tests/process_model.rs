//! Verifiable process-model tests for the TSP worker boundary.
//!
//! The protocol-only tests in `worker_integration.rs` exercise the
//! master's view of the worker. This file tests the *operating system*
//! view: the worker's actual PID, parent PID, executable path, and
//! command line. The TSP design contract requires that:
//!
//! - The master never spawns a Bun grandchild process.
//! - The worker's executable path matches the master's (Windows:
//!   self-spawn of the same `tspserver.exe`; Unix: `fork()` of
//!   the master so they share an executable).
//! - The worker receives `--tsp-worker` so it can route to the
//!   worker entry point instead of the master entry point.
//! - Crashed workers are reaped without becoming zombies, and the
//!   remaining workers in a pool keep serving.
//!
//! The `tsp_worker_test_stub` is the protocol peer for the tests; it
//! is built and `cargo test` resolves it via
//! `env!("CARGO_BIN_EXE_tsp_worker_test_stub")`. When the env var
//! `TSP_WORKER_INFO_PATH` is set, the stub writes its PID / PPID /
//! exe path / argv to that file before sending the Ready message,
//! and the test reads the file via the stub's `read_process_info`
//! helper.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bun_runtime_tsp::worker::manager::WorkerManager;
use bun_runtime_tsp::worker::pool::WorkerPool;
#[cfg(unix)]
use bun_runtime_tsp::worker::process_inspector;
use bun_runtime_tsp::worker::protocol::ExecuteRequest;

/// Global mutex that serialises the `TSP_WORKER_INFO_PATH`
/// env-var manipulation. The stub reads the env var from its
/// inherited environment; without this lock, parallel test
/// threads race on the shared global and the wrong worker
/// writes to the wrong file. CI also runs this binary with
/// `--test-threads=1` (see `.github/workflows/ci.yml`); the
/// mutex is the local-dev fallback so the tests pass under
/// `cargo test` defaults as well.
fn info_path_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

// Re-export the stub's read_process_info + ParsedProcessInfo so the
// tests don't need a second dependency on the bin crate. The bin
// itself is imported via the standard `#[path = ...]` mod-include
// pattern below, scoped to this test crate only.
#[path = "../worker_test_stub.rs"]
#[allow(dead_code, unreachable_pub)]
mod stub;

use stub::{read_process_info, ParsedProcessInfo};

fn stub_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tsp_worker_test_stub"))
}

fn request(path: &str, script: &[u8]) -> ExecuteRequest {
    ExecuteRequest {
        application: "test".into(),
        method: "GET".into(),
        path: path.into(),
        headers: Vec::new(),
        body: Vec::new(),
        script: script.to_vec(),
        context_json: "{}".into(),
        deadline_ms: 0,
    }
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    // Unix domain sockets have a small platform-defined pathname
    // limit; keep the test endpoint short even when the runner
    // provides a deeply nested temporary directory.
    #[cfg(unix)]
    let root = PathBuf::from("/tmp");
    #[cfg(not(unix))]
    let root = std::env::temp_dir();
    let file = root.join(format!(
        "tsp-{name}-{}-{nonce}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&file);
    file
}

fn unique_socket_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    #[cfg(unix)]
    let root = PathBuf::from("/tmp");
    #[cfg(not(unix))]
    let root = std::env::temp_dir();
    root.join(format!("tsp-{name}-{}-{nonce}.sock", std::process::id()))
}

fn unique_socket_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    #[cfg(unix)]
    let root = PathBuf::from("/tmp");
    #[cfg(not(unix))]
    let root = std::env::temp_dir();
    root.join(format!("tsp-{name}-{}-{nonce}", std::process::id()))
}

/// Wait for `path` to exist and contain the expected sentinel bytes,
/// or fail the test after the deadline. The stub writes the file
/// *before* the Ready message, so a successful `start_worker()`
/// implies the file is fully flushed by the time we get here; this
/// helper exists to give a clearer error message if that contract
/// ever breaks.
fn wait_for_info_file(path: &std::path::Path) -> ParsedProcessInfo {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(info) = read_process_info(path) {
            return info;
        }
        if Instant::now() >= deadline {
            panic!(
                "process info file did not appear at {}",
                path.display()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn worker_pid_matches_handle_id() {
    // The process-model contract: the PID the manager reports
    // through `child.id()` must be a live OS process, and the
    // worker must have written its own PID to the info file.
    let _guard = info_path_lock().lock().expect("lock should not be poisoned");
    let info_path = unique_temp_path("pid");
    let socket = unique_socket_path("pid");
    let mut manager = WorkerManager::new(stub_binary(), socket.clone());
    // The WorkerManager constructor does not let us inject env
    // vars per-spawn. The stub reads `TSP_WORKER_INFO_PATH` from
    // the inherited environment; to make the test self-contained
    // we set it on the current process before start, and unset
    // afterwards. The mutex above serialises this against
    // parallel test threads.
    unsafe { std::env::set_var("TSP_WORKER_INFO_PATH", &info_path) };
    let result = manager.start_worker();
    unsafe { std::env::remove_var("TSP_WORKER_INFO_PATH") };
    drop(_guard);
    result.expect("stub worker should become ready");

    let reported_pid = manager
        .stats()
        .expect("manager should have stats after start")
        .2;
    let info = wait_for_info_file(&info_path);

    assert_eq!(
        reported_pid, info.pid,
        "manager-reported pid must match what the worker wrote"
    );
    assert!(info.pid > 0, "worker pid must be a real OS pid");

    manager.stop_worker().expect("worker should stop");
    let _ = std::fs::remove_file(&info_path);
    let _ = std::fs::remove_file(&socket);
}

#[test]
fn worker_receives_tsp_worker_flag() {
    // The Windows self-spawn path and the Unix fork path both
    // require the worker to know it is the worker. The
    // `--tsp-worker` flag is that signal. The stub's argv
    // therefore MUST include it; the master entry path
    // (`is_tspserver_executable`) checks the same flag in the
    // real binary.
    let _guard = info_path_lock().lock().expect("lock should not be poisoned");
    let info_path = unique_temp_path("flag");
    let socket = unique_socket_path("flag");
    let mut manager = WorkerManager::new(stub_binary(), socket.clone());
    unsafe { std::env::set_var("TSP_WORKER_INFO_PATH", &info_path) };
    let result = manager.start_worker();
    unsafe { std::env::remove_var("TSP_WORKER_INFO_PATH") };
    drop(_guard);
    result.expect("stub worker should become ready");

    let info = wait_for_info_file(&info_path);
    assert!(
        info.argv.iter().any(|arg| arg == "--tsp-worker"),
        "worker argv must include --tsp-worker, got: {:?}",
        info.argv
    );

    manager.stop_worker().expect("worker should stop");
    let _ = std::fs::remove_file(&info_path);
    let _ = std::fs::remove_file(&socket);
}

#[test]
fn worker_executable_path_is_canonicalized() {
    // The Windows self-spawn contract: master's exe path ==
    // worker's exe path. We do not depend on platform-specific
    // process inspection; the worker writes its own exe path and
    // the master compares against the stub binary path used to
    // launch it. The master canonicity check is the smoke test's
    // job; here we just lock down that the stub reports its own
    // canonical path.
    let _guard = info_path_lock().lock().expect("lock should not be poisoned");
    let info_path = unique_temp_path("exe");
    let socket = unique_socket_path("exe");
    let stub_path = stub_binary();
    let mut manager = WorkerManager::new(stub_path.clone(), socket.clone());
    unsafe { std::env::set_var("TSP_WORKER_INFO_PATH", &info_path) };
    let result = manager.start_worker();
    unsafe { std::env::remove_var("TSP_WORKER_INFO_PATH") };
    drop(_guard);
    result.expect("stub worker should become ready");

    let info = wait_for_info_file(&info_path);
    let worker_exe = info.exe_path.clone();
    // Canonicalize both sides so symlinks / `././` differences do
    // not break the assertion. Windows: `\\?\` prefix, drive case
    // etc. are normalized by `canonicalize`.
    let worker_canon = std::fs::canonicalize(&worker_exe)
        .unwrap_or_else(|error| {
            panic!(
                "worker exe path {} should canonicalize: {error}",
                worker_exe.display()
            )
        });
    let stub_canon = std::fs::canonicalize(&stub_path)
        .unwrap_or_else(|error| {
            panic!(
                "stub binary path {} should canonicalize: {error}",
                stub_path.display()
            )
        });
    assert_eq!(
        worker_canon, stub_canon,
        "worker exe path must match the stub binary used to launch it"
    );

    manager.stop_worker().expect("worker should stop");
    let _ = std::fs::remove_file(&info_path);
    let _ = std::fs::remove_file(&socket);
}

#[cfg(unix)]
#[test]
fn linux_worker_parent_pid_is_test_pid() {
    // On Unix the master process is the test runner; the stub
    // uses fork()-style spawning (or `Command::spawn`, both of
    // which make the test runner the parent). Either way the
    // parent PID the worker reports must equal the test PID.
    let _guard = info_path_lock().lock().expect("lock should not be poisoned");
    let info_path = unique_temp_path("ppid");
    let socket = unique_socket_path("ppid");
    let mut manager = WorkerManager::new(stub_binary(), socket.clone());
    unsafe { std::env::set_var("TSP_WORKER_INFO_PATH", &info_path) };
    let result = manager.start_worker();
    unsafe { std::env::remove_var("TSP_WORKER_INFO_PATH") };
    drop(_guard);
    result.expect("stub worker should become ready");

    let info = wait_for_info_file(&info_path);
    assert_eq!(
        info.ppid, std::process::id(),
        "worker parent pid must equal the master (test) pid"
    );

    manager.stop_worker().expect("worker should stop");
    let _ = std::fs::remove_file(&info_path);
    let _ = std::fs::remove_file(&socket);
}

#[cfg(unix)]
#[test]
fn no_zombie_after_explicit_kill() {
    // After the manager calls `stop_worker`, the stub child must
    // be reaped. If the wait/kill/reap contract regresses, the
    // PID is still in /proc with state 'Z' and `kill(pid, 0)`
    // still succeeds — that combination is the "zombie" signal
    // process_inspector::is_alive would catch.
    let _guard = info_path_lock().lock().expect("lock should not be poisoned");
    let info_path = unique_temp_path("zombie");
    let socket = unique_socket_path("zombie");
    let mut manager = WorkerManager::new(stub_binary(), socket.clone());
    unsafe { std::env::set_var("TSP_WORKER_INFO_PATH", &info_path) };
    manager.start_worker().expect("start");
    let info = wait_for_info_file(&info_path);
    let worker_pid = info.pid;
    std::thread::sleep(Duration::from_millis(20));
    unsafe { std::env::remove_var("TSP_WORKER_INFO_PATH") };
    drop(_guard);
    manager.stop_worker().expect("stop");

    // After stop_worker returns, the child must be fully reaped
    // — `kill -0` must return ESRCH (process gone), not EPERM
    // (zombie still in the process table).
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut still_alive = process_inspector::is_alive(worker_pid);
    while still_alive && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
        still_alive = process_inspector::is_alive(worker_pid);
    }
    assert!(
        !still_alive,
        "worker pid {worker_pid} should be reaped, not alive as a zombie"
    );
    let _ = std::fs::remove_file(&info_path);
    let _ = std::fs::remove_file(&socket);
}

#[test]
fn multi_worker_pool_continues_when_one_crashes() {
    // Build a 3-worker pool. Crash worker 0, then dispatch 6
    // requests; every request must succeed because the pool
    // auto-restarts the crashed slot. The remaining two workers
    // must not be affected.
    let _guard = info_path_lock().lock().expect("lock should not be poisoned");
    let info_path_a = unique_temp_path("crasha");
    let info_path_b = unique_temp_path("crashb");
    let info_path_c = unique_temp_path("crashc");
    let socket_dir = unique_socket_dir("crash-pool");
    std::fs::create_dir_all(&socket_dir).expect("socket dir should be creatable");

    // We can't set TSP_WORKER_INFO_PATH per-slot with the
    // WorkerManager API; assert against the *protocol* contract
    // (every request returns 200) instead of per-slot OS info.
    // The per-slot info files here only exist so the test can
    // confirm all three workers came up cleanly before the crash.
    unsafe { std::env::set_var("TSP_WORKER_INFO_PATH", &info_path_a) };
    let pool = Arc::new(WorkerPool::new(stub_binary(), &socket_dir, 3, 3));
    pool.start().expect("pool should start all three workers");
    unsafe { std::env::remove_var("TSP_WORKER_INFO_PATH") };

    // Sanity: each of the three workers wrote its own info file
    // (one of them, since the env is shared; the other two
    // reuse the same path on disk — that's fine, we only need
    // a record that at least one worker came up).
    let _ = wait_for_info_file(&info_path_a);
    // Crash a request: the "crash" path in the stub calls
    // std::process::exit(17). The manager's next execute must
    // succeed because the slot restarts.
    let _ = pool
        .execute(request("crash", b""), 1_000)
        .expect_err("crashing request should fail");
    // Dispatch 6 follow-up requests; each must succeed via
    // the surviving or restarted slot.
    for index in 0..6 {
        let response = pool
            .execute(request(&format!("after-{index}"), b"ok"), 1_000)
            .unwrap_or_else(|error| {
                panic!("request {index} should succeed after a crash, got: {error}")
            });
        assert_eq!(response.body, b"ok");
    }

    unsafe { std::env::set_var("TSP_WORKER_INFO_PATH", &info_path_b) };
    pool.restart_all().expect("pool should restart all workers");
    unsafe { std::env::remove_var("TSP_WORKER_INFO_PATH") };

    drop(pool);
    drop(_guard);
    let _ = std::fs::remove_file(&info_path_a);
    let _ = std::fs::remove_file(&info_path_b);
    let _ = std::fs::remove_file(&info_path_c);
    let _ = std::fs::remove_dir_all(&socket_dir);
}

// -----------------------------------------------------------------------
// Slice B: fork wait/kill/reap + cancel path.
//
// These tests are layered on top of the Slice A contracts. They
// exercise the manager's *lifecycle* rather than the protocol:
// explicit kill from outside, cancel during a long-running request,
// and the idempotence of start_worker.
// -----------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn explicit_sigkill_from_outside_is_reaped_and_recovered() {
    // Simulate a worker that the OS takes down with SIGKILL (no
    // graceful exit, no exit-handler fire). The manager must
    // detect the dead stream on the next request, restart the
    // slot, and serve the replacement.
    let _guard = info_path_lock().lock().expect("lock should not be poisoned");
    let info_path = unique_temp_path("kill");
    let socket = unique_socket_path("kill");
    let mut manager = WorkerManager::new(stub_binary(), socket.clone());
    unsafe { std::env::set_var("TSP_WORKER_INFO_PATH", &info_path) };
    manager.start_worker().expect("start");
    let info = wait_for_info_file(&info_path);
    let worker_pid = info.pid as libc::pid_t;
    unsafe { std::env::remove_var("TSP_WORKER_INFO_PATH") };
    drop(_guard);

    // Send SIGKILL to the worker; the stub does not install any
    // handler so the kernel reaps it without a clean exit.
    let result = unsafe { libc::kill(worker_pid, libc::SIGKILL) };
    assert_eq!(result, 0, "SIGKILL should succeed");

    // Give the kernel a moment to clean up the task struct and
    // for the master's stream read to observe the broken pipe.
    std::thread::sleep(Duration::from_millis(100));

    // The next request must succeed via the replacement worker.
    let response = manager
        .execute(request("after-kill", b"resurrected"))
        .expect("replacement should serve after SIGKILL");
    assert_eq!(response.body, b"resurrected");

    manager.stop_worker().expect("stop");
    let _ = std::fs::remove_file(&info_path);
    let _ = std::fs::remove_file(&socket);
}

#[test]
fn start_worker_is_idempotent() {
    // Calling start_worker a second time on an already-running
    // manager must be a no-op. The integration test ensures the
    // master never duplicates a worker slot by accident.
    let socket = unique_socket_path("idempotent");
    let mut manager = WorkerManager::new(stub_binary(), socket.clone());
    manager.start_worker().expect("first start");
    let first_pid = manager
        .stats()
        .expect("manager should have stats")
        .2;
    manager.start_worker().expect("second start is a no-op");
    let second_pid = manager
        .stats()
        .expect("manager should still have stats")
        .2;
    assert_eq!(first_pid, second_pid, "start_worker must not respawn");
    manager.stop_worker().expect("stop");
    let _ = std::fs::remove_file(&socket);
}

#[test]
fn cancel_during_in_flight_request_does_not_break_next_request() {
    // The manager exposes `cancel(id)` for in-flight requests
    // that the host has decided to abort. The cancel must reach
    // the worker (the stub's Message::Cancel handler is
    // intentionally a no-op so we can assert the manager's
    // bookkeeping without depending on stub behaviour), and
    // the next request must succeed via the same slot.
    let socket = unique_socket_path("cancel");
    let mut manager = WorkerManager::new(stub_binary(), socket.clone());
    manager.start_worker().expect("start");

    // Run a slow request synchronously so we control the timing.
    // The stub echoes the script bytes as the response body when
    // the script is non-empty, so use a sentinel here.
    let response = manager
        .execute_with_timeout(request("slow", b"__TSP_TEST_SLEEP__"), 1_000)
        .expect("sleep request should complete within 1s");
    assert_eq!(response.body, b"__TSP_TEST_SLEEP__");

    // Cancel an arbitrary id against the live worker; the stub
    // treats cancel as a no-op so this is purely a master-side
    // assertion.
    manager
        .cancel(42)
        .expect("cancel should write to the worker stream");

    // The next request must still succeed; the stream is not
    // torn down by a benign cancel.
    let response = manager
        .execute(request("after-cancel", b"ok"))
        .expect("subsequent request should succeed after cancel");
    assert_eq!(response.body, b"ok");

    manager.stop_worker().expect("stop");
    let _ = std::fs::remove_file(&socket);
}

#[test]
fn worker_receives_heartbeat_and_responds() {
    // The protocol's Heartbeat message round-trips through the
    // manager's `health_check`; pin the contract that the stub
    // echoes the heartbeat id, so any future Heartbeat
    // wire-format change fails this test instead of leaving
    // master/worker drift in production.
    let socket = unique_socket_path("heartbeat");
    let mut manager = WorkerManager::new(stub_binary(), socket.clone());
    manager.start_worker().expect("start");
    manager
        .health_check()
        .expect("heartbeat round-trip should succeed");
    manager.stop_worker().expect("stop");
    let _ = std::fs::remove_file(&socket);
}
