use bun_runtime_tsp::worker::manager::{ManagerError, WorkerManager};
use bun_runtime_tsp::worker::pool::{PoolError, WorkerPool};
use bun_runtime_tsp::worker::protocol::ExecuteRequest;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn stub_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tsp_worker_test_stub"))
}

fn socket_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    // Unix domain sockets have a small platform-defined pathname limit. Keep
    // the test endpoint short even when the runner provides a deeply nested
    // temporary directory; production socket paths remain configurable.
    #[cfg(unix)]
    let root = PathBuf::from("/tmp");
    #[cfg(not(unix))]
    let root = std::env::temp_dir();
    root.join(format!("tsp-{name}-{}-{nonce}.sock", std::process::id()))
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

#[test]
fn manager_reuses_one_worker_for_multiple_requests() {
    let socket = socket_path("reuse");
    let mut manager = WorkerManager::new(stub_binary(), socket.clone());
    manager
        .start_worker()
        .expect("stub worker should become ready");
    let first = manager
        .execute(request("first", b"one"))
        .expect("first request should succeed");
    let second = manager
        .execute(request("second", b"two"))
        .expect("second request should succeed");
    assert_eq!(first.body, b"one");
    assert_eq!(second.body, b"two");
    assert_eq!(manager.stats().expect("worker stats should exist").1, 2);
    manager.health_check().expect("heartbeat should succeed");
    manager.stop_worker().expect("worker should stop");
    assert!(!socket.exists());
}

#[test]
fn crashed_worker_is_replaced_before_next_request() {
    let mut manager = WorkerManager::new(stub_binary(), socket_path("crash"));
    manager
        .start_worker()
        .expect("stub worker should become ready");
    let error = manager
        .execute(request("crash", b""))
        .expect_err("crash should fail");
    assert!(matches!(
        error,
        ManagerError::WorkerExited | ManagerError::Protocol(_)
    ));
    let response = manager
        .execute(request("after-restart", b"recovered"))
        .expect("replacement should serve");
    assert_eq!(response.body, b"recovered");
}

#[test]
fn timeout_restarts_a_stuck_worker() {
    let mut manager = WorkerManager::new(stub_binary(), socket_path("timeout"));
    manager
        .start_worker()
        .expect("stub worker should become ready");
    let error = manager
        .execute_with_timeout(request("sleep", b"__TSP_TEST_SLEEP__"), 25)
        .expect_err("slow request should time out");
    assert!(matches!(error, ManagerError::WorkerTimeout));
    let response = manager
        .execute(request("after-timeout", b"alive"))
        .expect("replacement should serve");
    assert_eq!(response.body, b"alive");
}

#[test]
fn pool_applies_admission_backpressure() {
    let pool = Arc::new(WorkerPool::new(stub_binary(), socket_path("pool"), 1, 1));
    pool.start().expect("pool worker should start");
    let running = Arc::clone(&pool);
    let thread =
        thread::spawn(move || running.execute(request("sleep", b"__TSP_TEST_SLEEP__"), 500));
    thread::sleep(Duration::from_millis(30));
    let error = pool
        .execute(request("queued", b"queued"), 20)
        .expect_err("full pool should apply backpressure");
    assert!(matches!(error, PoolError::Backpressure));
    thread
        .join()
        .expect("worker thread should join")
        .expect("slow request should finish");
    pool.restart_all().expect("pool should remain restartable");
}
