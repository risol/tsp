//! Boot-order contract test for the v2.4 host.
//!
//! The TSP v2.4 design requires that the worker pool is started
//! before the HTTP listener accepts the first request. On Unix
//! this is non-negotiable because the master uses `fork()` while
//! only the boot thread exists, but the contract applies on every
//! platform so the same startup ordering is consistent.
//!
//! This test exercises the real `tspserver_v2` binary rather than
//! the `WorkerPool` API directly: the contract is about the order
//! of side effects in `bin/tspserver_v2.rs`, not about the pool
//! itself. The host prints the relevant markers to stderr; the
//! test boots the binary, sends SIGTERM, and asserts the markers
//! appeared in the right relative order.
//!
//! The test is gated on the existence of a built `tspserver_v2`
//! binary at `dist/tsp-v2/tspserver_v2[.exe]`. The CI job that
//! already builds the runtime (`smoke-linux`, plus the
//! `release.yml` matrix) is the right place to run this; running
//! it without a prior build skips the assertions.

use std::io::Read;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

static MASTER_BINARY: OnceLock<Option<PathBuf>> = OnceLock::new();

fn locate_master() -> Option<&'static PathBuf> {
    MASTER_BINARY
        .get_or_init(|| locate_master_inner())
        .as_ref()
}

fn locate_master_inner() -> Option<PathBuf> {
    // Walk from the test target's working directory up to the
    // workspace root looking for `dist/tsp-v2/tspserver_v2[.exe]`.
    // The CI job that runs this test is responsible for
    // populating that directory before invoking cargo test.
    let candidates = if cfg!(windows) {
        ["dist\\tsp-v2\\tspserver_v2.exe", "..\\..\\..\\..\\dist\\tsp-v2\\tspserver_v2.exe"]
    } else {
        ["dist/tsp-v2/tspserver_v2", "../../../../dist/tsp-v2/tspserver_v2"]
    };
    for relative in candidates {
        let path = PathBuf::from(relative);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// Markers emitted by `bin/tspserver_v2.rs` in stable order. The
/// strings are quoted from the source; if a future slice renames
/// one of them the test fails with a clear error so the contract
/// change is conscious.
const MARKER_WORKER: &str = "v2.4 embedded worker enabled";
const MARKER_WATCHER: &str = "watcher polling";
const MARKER_LISTEN: &str = "listening on";

#[test]
fn worker_pool_starts_before_watcher_and_listener() {
    let Some(master) = locate_master() else {
        eprintln!(
            "skipping: tspserver_v2 binary not found under dist/tsp-v2/ \
             (run ./tsp.sh build:host first)"
        );
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-start-order-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir should be creatable");
    // Minimal route the host can scan without crashing.
    std::fs::write(
        routes_dir.join("index.tsp"),
        b"export function GET() { return null; }\n",
    )
    .expect("index.tsp should be writable");

    // Pick a port unlikely to clash; if it does the test fails
    // fast on the listener accept anyway.
    let port: u16 = 29_000 + (std::process::id() as u16 % 1_000);

    let info = std::process::Command::new(master)
        .env("TSP_PORT", port.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn");
    let pid = info.id();
    let mut stderr = info.stderr.expect("stderr should be piped");

    // Give the master up to 5s to print all three markers, then
    // terminate it; the test asserts on whatever was printed.
    let mut buffer = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut last_read = std::time::Instant::now();
    while std::time::Instant::now() < deadline {
        if last_read.elapsed() >= Duration::from_millis(100) {
            let mut chunk = [0u8; 1024];
            if let Ok(read) = stderr.read(&mut chunk) {
                if read > 0 {
                    buffer.push_str(&String::from_utf8_lossy(&chunk[..read]));
                }
            }
            last_read = std::time::Instant::now();
        }
        if buffer.contains(MARKER_LISTEN) {
            // The listener line is the last of the three; once
            // it appears we know we have everything we need.
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Best-effort terminate; the assertion below is the real
    // signal, the kill is just to keep CI from hanging.
    let _ = terminate(pid);
    let _ = std::fs::remove_dir_all(&temp_root);

    let worker_at = buffer.find(MARKER_WORKER);
    let watcher_at = buffer.find(MARKER_WATCHER);
    let listener_at = buffer.find(MARKER_LISTEN);

    assert!(
        worker_at.is_some(),
        "master never printed the worker-enabled marker; stderr was: {buffer:?}"
    );
    assert!(
        watcher_at.is_some(),
        "master never printed the watcher marker; stderr was: {buffer:?}"
    );
    assert!(
        listener_at.is_some(),
        "master never printed the listener marker; stderr was: {buffer:?}"
    );

    let worker_at = worker_at.unwrap();
    let watcher_at = watcher_at.unwrap();
    let listener_at = listener_at.unwrap();
    assert!(
        worker_at < watcher_at,
        "worker marker ({worker_at}) must appear before watcher marker ({watcher_at}); full stderr: {buffer:?}"
    );
    assert!(
        watcher_at < listener_at,
        "watcher marker ({watcher_at}) must appear before listener marker ({listener_at}); full stderr: {buffer:?}"
    );
}

#[cfg(unix)]
fn terminate(pid: u32) -> std::io::Result<()> {
    // Safety: `kill(pid, SIGTERM)` is a documented request to
    // shut down; the master catches it via the default handler.
    let result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn terminate(pid: u32) -> std::io::Result<()> {
    // The Windows shutdown is driven by `GenerateConsoleCtrlEvent`
    // or by killing the process; the smoke scripts use the
    // latter. Avoid pulling in extra Windows surface here.
    let output = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Multi-route dispatch regression test (BUG-0001)
//
// Each request to the running server must return the body produced by
// the route that the URL resolves to, not a cached body from a
// previous request. The original bug cached the first request's wrap
// preamble in Bun's `bun:main` module-registry entry, so every later
// request aliased to whichever .tsp the first request hit. The fix
// lives in `tsp_worker.rs::execute_path_with_api_lock` (call
// `vm.clear_entry_point()` before every `load_entry_point`).
//
// We use `TSP_WORKER_COUNT=1` so the test is deterministic: a single
// worker VM means the alias bug, if present, is 100% reproducible.
// A second test with `>=2` workers would cover pool-rotation cases
// but is flaky on shared CI hardware and is left to manual runs.
// ---------------------------------------------------------------------------

const INDEX_BODY: &str = "BUG-0001-INDEX-MARKER";
const TIME_BODY: &str = "BUG-0001-TIME-MARKER";
const INDEX_TSP: &str = "export function GET() {\
  return <p>BUG-0001-INDEX-MARKER</p>;\
}\
";
const TIME_TSP: &str = "export function GET() {\
  return <p>BUG-0001-TIME-MARKER</p>;\
}\
";

#[test]
fn multi_route_dispatch_does_not_alias_to_first_request() {
    let Some(master) = locate_master() else {
        eprintln!(
            "skipping: tspserver_v2 binary not found under dist/tsp-v2/ \
             (run ./tsp.sh build:host first)"
        );
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-multi-route-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir should be creatable");
    // Two distinct .tsp files, each with a body fingerprint the
    // assertion can grep for. The pre-fix bug returned the first
    // request's INDEX_BODY for /time as well.
    std::fs::write(routes_dir.join("index.tsp"), INDEX_TSP).expect("write index.tsp");
    std::fs::write(routes_dir.join("time.tsp"), TIME_TSP).expect("write time.tsp");

    let port: u16 = 30_000 + (std::process::id() as u16 % 1_000);

    let mut child = std::process::Command::new(master)
        .env("TSP_PORT", port.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn");

    // Wait for the listener to print "listening on" before
    // sending requests; otherwise the first probe races the boot
    // and surfaces a confusing ECONNREFUSED.
    wait_for_marker(&mut child, "listening on", Duration::from_secs(10));

    // Send the requests in a non-trivial order: time FIRST, then
    // index. The pre-fix bug would alias both responses to the
    // first request's body (TIME_BODY), regardless of which route
    // was the first hit.
    let time_first = http_get(port, "/time", Duration::from_secs(5));
    let index_first = http_get(port, "/", Duration::from_secs(5));
    let time_again = http_get(port, "/time", Duration::from_secs(5));

    let _ = terminate(child.id());

    assert!(
        time_first.contains(TIME_BODY),
        "/time first request must contain {TIME_BODY:?}, got: {time_first:?} \
         (regression of BUG-0001: aliased to first request's wrap)"
    );
    assert!(
        index_first.contains(INDEX_BODY),
        "/ first request must contain {INDEX_BODY:?}, got: {index_first:?}"
    );
    assert!(
        time_again.contains(TIME_BODY),
        "/time second request must contain {TIME_BODY:?}, got: {time_again:?} \
         (regression of BUG-0001: cached bun:main module replayed from \
         first request)"
    );
    // Defence-in-depth: the two route fingerprints must never
    // appear in each other's body. If a future regression makes
    // one route include the other's text, the test fails with a
    // clear "cross-contamination" message instead of an opaque
    // substring assertion.
    assert!(
        !time_first.contains(INDEX_BODY) && !time_again.contains(INDEX_BODY),
        "/time body leaked the / marker (cross-route contamination): {time_first:?} / {time_again:?}"
    );
    assert!(
        !index_first.contains(TIME_BODY),
        "/ body leaked the /time marker (cross-route contamination): {index_first:?}"
    );

    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

/// Spawn the real binary, then read its stderr until `marker` shows
/// up or the deadline elapses. Returns silently on hit; the caller
/// proceeds to fire HTTP probes.
fn wait_for_marker(
    child: &mut std::process::Child,
    marker: &str,
    deadline: Duration,
) {
    let Some(mut stderr) = child.stderr.take() else {
        panic!("stderr was not piped for the master process");
    };
    let start = std::time::Instant::now();
    let mut buffer = String::new();
    while start.elapsed() < deadline {
        let mut tmp = [0u8; 1024];
        match stderr.read(&mut tmp) {
            Ok(0) => std::thread::sleep(Duration::from_millis(20)),
            Ok(n) => {
                buffer.push_str(&String::from_utf8_lossy(&tmp[..n]));
                if buffer.contains(marker) {
                    // Put stderr back so the parent can drain it
                    // (and we don't drop the marker's full line in
                    // the OS pipe buffer).
                    child.stderr = Some(stderr);
                    return;
                }
            }
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    panic!(
        "master never printed {:?} within {:?}; partial stderr: {buffer:?}",
        marker, deadline
    );
}

/// Minimal HTTP/1.1 GET against `http://127.0.0.1:{port}{path}`. We
/// speak the protocol directly instead of pulling in a client lib
/// so the test stays self-contained.
fn http_get(port: u16, path: &str, deadline: Duration) -> String {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let start = std::time::Instant::now();
    let mut stream = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => break s,
            Err(_) if start.elapsed() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => panic!("connect to 127.0.0.1:{port} failed: {e}"),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set_read_timeout");
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .expect("write request");
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .expect("read response");
    // Strip the HTTP status line + headers; tests assert on the
    // body fingerprint only, which is what BUG-0001 actually
    // affected (the wrap preamble's `__tspEmbeddedResponse`).
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    body
}

// ---------------------------------------------------------------------------
// nanoid runtime integration test (slice 17a)
//
// The TSP v2 wrap preamble inlines the nanoid 5.1.6 source via
// `include_str!` and exposes `globalThis.nanoid()` (and
// `customAlphabet`, `customRandom`, `random`) so pages can call
// them without an import step. We assert this end-to-end by
// spinning the real binary against a temp routes dir that
// contains `nanoid.tsp`, sending the documented GET and POST
// requests, and checking the body shape + length.
// ---------------------------------------------------------------------------

const NANOID_TSP: &str = r#"
// Generated by the nanoid integration test in
// bun/src/runtime/tsp/tests/start_order.rs. Mirrors the
// production `routes/nanoid.tsp` shape; the regression test
// inlines this rather than copying the file so the assertion
// is self-contained.
//
// Plan §16.4: framework API must be explicitly imported (or
// accessed via Context), NOT on globalThis. The page reaches
// nanoid via `import { nanoid } from "tsp:server"`; the host
// rewriter turns that into `const { nanoid } = __tspServer;`
// against the frozen object the wrap preamble built.
import { nanoid } from "tsp:server";

export function GET(_ctx) {
  return new Response(nanoid(), {
    status: 200,
    headers: { "content-type": "text/plain" },
  });
}

export async function POST(ctx) {
  const body = await ctx.request.text();
  let size = 21;
  if (body) {
    try {
      const parsed = JSON.parse(body);
      if (typeof parsed.size === "number" && parsed.size > 0) {
        size = parsed.size;
      }
    } catch {
      // body wasn't JSON, ignore
    }
  }
  return new Response(nanoid(size), {
    status: 200,
    headers: { "content-type": "text/plain", "x-demo": "slice17a" },
  });
}
"#;

const NANOID_URL_ALPHABET: &[u8] =
    b"useandom-26T198340PX75pxJACKVERYMINDBUSHWOLF_GQZbfghjklqvwyzrict";

#[test]
fn nanoid_runtime_compiled_into_wrap_serves_distinct_ids() {
    let Some(master) = locate_master() else {
        eprintln!(
            "skipping: tspserver_v2 binary not found under dist/tsp-v2/ \
             (run ./tsp.sh build:host first)"
        );
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-nanoid-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir should be creatable");
    std::fs::write(routes_dir.join("nanoid.tsp"), NANOID_TSP)
        .expect("nanoid.tsp should be writable");

    let port: u16 = 31_500 + (std::process::id() as u16 % 500);
    let mut child = std::process::Command::new(master)
        .env("TSP_PORT", port.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn");

    wait_for_marker(&mut child, "listening on", Duration::from_secs(10));

    // GET /nanoid three times in a row. Each must be 21 chars and
    // every response must be a different id (so we know the worker
    // is not aliasing or returning a cached envelope).
    let mut ids: Vec<String> = Vec::new();
    for _ in 0..3 {
        let body = http_get(port, "/nanoid", Duration::from_secs(5));
        assert_eq!(body.len(), 21, "default nanoid must be 21 chars, got {body:?}");
        assert!(
            body.bytes().all(|b| NANOID_URL_ALPHABET.contains(&b)),
            "default nanoid must only use url-alphabet chars, got {body:?}"
        );
        ids.push(body);
    }
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(
        unique.len(),
        3,
        "three sequential GETs must produce three distinct ids; got {ids:?}"
    );

    // POST /nanoid with explicit size. Each response length must
    // match the requested size and use only url-alphabet chars.
    for size in [8u32, 16, 32] {
        let body = http_post_json(
            port,
            "/nanoid",
            &format!(r#"{{"size":{size}}}"#),
            Duration::from_secs(5),
        );
        assert_eq!(
            body.len(),
            size as usize,
            "POST size={size} must return {size}-char id, got {body:?}"
        );
        assert!(
            body.bytes().all(|b| NANOID_URL_ALPHABET.contains(&b)),
            "size={size} id must only use url-alphabet chars, got {body:?}"
        );
    }

    // POST /nanoid with invalid JSON body must fall back to the
    // default 21-char size (the page catches and ignores the parse
    // error).
    let body = http_post_json(port, "/nanoid", "not json", Duration::from_secs(5));
    assert_eq!(body.len(), 21, "invalid JSON must use default 21, got {body:?}");

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

/// Minimal HTTP/1.1 POST with a Content-Type and JSON body.
/// Same shape as `http_get` but writes the body and waits for
/// the response.
fn http_post_json(port: u16, path: &str, body: &str, deadline: Duration) -> String {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let start = std::time::Instant::now();
    let mut stream = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => break s,
            Err(_) if start.elapsed() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => panic!("connect to 127.0.0.1:{port} failed: {e}"),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set_read_timeout");
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .expect("write request");
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .expect("read response");
    raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string()
}
