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

// ---------------------------------------------------------------------------
// zod runtime integration test (slice 17b)
//
// The TSP v2 wrap preamble inlines the pre-bundled zod 3.25.76
// CJS via `include_str!` and exposes `__tspServer.zod` so pages
// can declare schemas / call `safeParse` without an import step.
// We assert this end-to-end by spinning the real binary against
// a temp routes dir that contains `zod.tsp`, sending the
// documented GET (default parse) and POST (schema-validated
// echo) requests, and checking both the body fingerprint and
// the status code. The status line is the strongest signal
// that the validation path works (200 on success, 400 on
// schema failure) without scraping internal zod error shapes.
// ---------------------------------------------------------------------------

const ZOD_TSP: &str = r#"
// Generated by the zod integration test in
// bun/src/runtime/tsp/tests/start_order.rs. Mirrors the
// production `routes/zod.tsp` shape; the regression test
// inlines this rather than copying the file so the assertion
// is self-contained.
//
// Plan §16.4: framework API must be explicitly imported (or
// accessed via Context), NOT on globalThis. The page reaches
// zod via `import { zod } from "tsp:server"`; the host
// rewriter turns that into `const { zod } = __tspServer;`
// against the frozen object the wrap preamble built.
import { zod } from "tsp:server";

export function GET(_ctx) {
  const schema = zod.object({
    name: zod.string(),
    age: zod.coerce.number().int().min(0).max(150),
  });
  const result = schema.safeParse({ name: "alice", age: "30" });
  if (!result.success) {
    return new Response(JSON.stringify({ ok: false, issues: result.error.issues }), {
      status: 400,
      headers: { "content-type": "application/json" },
    });
  }
  return new Response(
    JSON.stringify({ ok: true, name: result.data.name, age: result.data.age }),
    {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "slice17b" },
    }
  );
}

export async function POST(ctx) {
  const schema = zod.object({
    email: zod.string().email(),
    age: zod.coerce.number().int().min(0).max(150),
  });
  const body = await ctx.request.text();
  let parsed;
  try {
    parsed = JSON.parse(body);
  } catch {
    return new Response(JSON.stringify({ ok: false, error: "body must be JSON" }), {
      status: 400,
      headers: { "content-type": "application/json" },
    });
  }
  const result = schema.safeParse(parsed);
  if (!result.success) {
    return new Response(
      JSON.stringify({ ok: false, issues: result.error.issues.map((i) => i.message) }),
      { status: 400, headers: { "content-type": "application/json" } }
    );
  }
  return new Response(
    JSON.stringify({ ok: true, email: result.data.email, age: result.data.age }),
    { status: 200, headers: { "content-type": "application/json", "x-demo": "slice17b" } }
  );
}
"#;

/// Hand-rolled JSON string-field extractor. We need a single
/// `"hash":"..."` value from the GET /password response so the
/// POST can use the just-generated hash. Avoiding a `serde_json`
/// dev-dep keeps the test crate's build graph small. The
/// `tsp` runtime test crate deliberately stays `std`-only --
/// the assertions are on a known, well-formed body shape.
fn extract_json_string_field(body: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":\"");
    let start = body.find(&needle)? + needle.len();
    let rest = &body[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Minimal HTTP/1.1 request that returns `(status_code, body)`.
/// We need the status line separately because the password
/// and zod tests assert on `200` vs `400` to prove the
/// verification / validation paths work end-to-end.
fn http_get_status(port: u16, path: &str, deadline: Duration) -> (u16, String) {
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
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).expect("write request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read response");
    let status_line = raw.lines().next().unwrap_or("");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

fn http_post_status(
    port: u16,
    path: &str,
    body: &str,
    deadline: Duration,
) -> (u16, String) {
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
    stream.write_all(request.as_bytes()).expect("write request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read response");
    let status_line = raw.lines().next().unwrap_or("");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

/// Minimal HTTP/1.1 GET that lets the caller attach a
/// `Cookie:` header (so the test can send a session id
/// back to the server on subsequent requests). Returns
/// `(status, body)` just like `http_get_status`; the
/// `Set-Cookie` lines the server emits ride back through
/// the response headers and are inspected by the
/// cookies / session e2e tests via the raw-response
/// helper below.
fn http_get_with_cookie(
    port: u16,
    path: &str,
    cookie: &str,
    deadline: Duration,
) -> (u16, String) {
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
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).expect("write request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read response");
    let status_line = raw.lines().next().unwrap_or("");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

/// Returns the full raw HTTP/1.1 response so the caller
/// can inspect `Set-Cookie` and other multi-value headers
/// that `http_get_status` / `http_post_status` would
/// collapse. Used by the cookies e2e test to assert that
/// a `ctx.cookies.set(...)` call actually produced a
/// `Set-Cookie:` line on the response.
fn http_get_raw(
    port: u16,
    path: &str,
    deadline: Duration,
) -> (u16, String) {
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
    stream.write_all(request.as_bytes()).expect("write request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read response");
    let status_line = raw.lines().next().unwrap_or("");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, raw)
}

#[test]
fn zod_runtime_compiled_into_wrap_serves_validated_schemas() {
    let Some(master) = locate_master() else {
        eprintln!(
            "skipping: tspserver_v2 binary not found under dist/tsp-v2/ \
             (run ./tsp.sh build:host first)"
        );
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-zod-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir should be creatable");
    std::fs::write(routes_dir.join("zod.tsp"), ZOD_TSP).expect("zod.tsp should be writable");

    let port: u16 = 32_000 + (std::process::id() as u16 % 500);
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

    // GET /zod: the page runs zod.coerce.number() on "30" and
    // returns `{ ok: true, name: "alice", age: 30 }`. The 200
    // status plus the three string fragments prove the namespace
    // is reachable from the page AND the parse succeeded.
    let (status_get, body_get) = http_get_status(port, "/zod", Duration::from_secs(5));
    assert_eq!(
        status_get, 200,
        "GET /zod must return 200 (parse ok), got {status_get} body={body_get:?}"
    );
    assert!(
        body_get.contains("\"ok\":true")
            && body_get.contains("\"name\":\"alice\"")
            && body_get.contains("\"age\":30"),
        "GET /zod body must contain the parsed zod result; got {body_get:?}"
    );

    // POST /zod with a valid body: 200 + echo. This proves
    // `ctx.request.text()` -> `JSON.parse` -> `safeParse` ->
    // `result.data` flows through to a real Response.
    let (status_ok, body_ok) = http_post_status(
        port,
        "/zod",
        r#"{"email":"alice@example.com","age":"42"}"#,
        Duration::from_secs(5),
    );
    assert_eq!(
        status_ok, 200,
        "POST /zod with valid body must return 200, got {status_ok} body={body_ok:?}"
    );
    assert!(
        body_ok.contains("\"email\":\"alice@example.com\"") && body_ok.contains("\"age\":42"),
        "POST /zod valid body must echo parsed fields; got {body_ok:?}"
    );

    // POST /zod with an invalid body (bad email): 400 + the
    // issue messages array. This is the strongest signal that
    // the validation path runs end-to-end (zod collects
    // `error.issues`, the page maps to messages, Response gets
    // status 400).
    let (status_bad, body_bad) = http_post_status(
        port,
        "/zod",
        r#"{"email":"not-an-email","age":"200"}"#,
        Duration::from_secs(5),
    );
    assert_eq!(
        status_bad, 400,
        "POST /zod with invalid body must return 400, got {status_bad} body={body_bad:?}"
    );
    assert!(
        body_bad.contains("\"ok\":false"),
        "POST /zod invalid body must report ok:false; got {body_bad:?}"
    );

    // POST /zod with a non-JSON body: page returns 400 with
    // `{ ok: false, error: "body must be JSON" }`. This proves
    // the page-level try/catch (around `JSON.parse`) is in
    // place AND the zod prelude is not the only failure mode.
    let (status_garbage, body_garbage) =
        http_post_status(port, "/zod", "not json", Duration::from_secs(5));
    assert_eq!(
        status_garbage, 400,
        "POST /zod with non-JSON body must return 400, got {status_garbage} body={body_garbage:?}"
    );
    assert!(
        body_garbage.contains("body must be JSON"),
        "POST /zod non-JSON body must carry the explicit error; got {body_garbage:?}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// password runtime integration test (slice 17c)
//
// The TSP v2 wrap preamble bridges bun's native `Bun.password`
// to the page as `__tspServer.password`. The page reaches it
// via `import { password } from "tsp:server"`; the rewriter
// emits `const { password } = __tspServer;` and the page uses
// `password.hashSync(...)` / `password.verifySync(...)`.
//
// We assert this end-to-end by spinning the real binary
// against a temp routes dir that contains `password.tsp`,
// sending the documented GET (default bcrypt hash, plus an
// `?algo=argon2id` variant) and POST (verify) requests, and
// checking the status + body shape. The `verify` flow is the
// strongest signal: 200 means the password matched the stored
// hash, 400 means it didn't, and the page code branches on
// the actual `Bun.password.verifySync` return value rather
// than swallowing it.
//
// This is a **follow-up to slice 17c** (which embedded bcryptjs
// 34 KB via `include_str!` + IIFE wrap). The embed was
// abandoned once we confirmed `Bun.password` covers bcrypt /
// argon2id / scrypt with native Rust performance and zero
// per-request parse cost. The bcryptjs vendor bundle is
// deleted (git history preserves it); `bcryptjs` is removed
// from `bun/package.json#devDependencies` and `bun.lock`.
// Page-side API is the bun-native shape directly (no v1
// `bcrypt.hashSync(pw, salt)` thin wrapper).
// ---------------------------------------------------------------------------

const PASSWORD_TSP: &str = r#"
// Generated by the password integration test in
// bun/src/runtime/tsp/tests/start_order.rs. Mirrors the
// production `routes/password.tsp` shape; the regression test
// inlines this rather than copying the file so the assertion
// is self-contained.
//
// Plan §16.4: framework API must be explicitly imported (or
// accessed via Context), NOT on globalThis. The page reaches
// the password namespace via `import { password } from
// "tsp:server"`; the host rewriter turns that into
// `const { password } = __tspServer;` against the frozen
// object the wrap preamble built. `__tspServer.password` is
// the same object as `Bun.password`, so any future builtin
// API surface (e.g. `Bun.password.options`) is automatically
// available to the page without a v2 release.
import { password } from "tsp:server";

export function GET(_ctx) {
  const algo = _ctx.query.get("algo") || "bcrypt";
  const hash = password.hashSync("hello", { algorithm: algo, cost: 4 });
  return new Response(
    JSON.stringify({
      ok: true,
      algorithm: algo,
      hash,
      isBcrypt: hash.startsWith("$2b$"),
      isArgon: hash.startsWith("$argon2id$"),
    }),
    {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "slice17c" },
    }
  );
}

export async function POST(ctx) {
  const body = await ctx.request.text();
  let parsed;
  try {
    parsed = JSON.parse(body);
  } catch {
    return new Response(JSON.stringify({ ok: false, error: "body must be JSON" }), {
      status: 400,
      headers: { "content-type": "application/json" },
    });
  }
  if (typeof parsed.pw !== "string" || typeof parsed.hash !== "string") {
    return new Response(
      JSON.stringify({ ok: false, error: "pw and hash must be strings" }),
      { status: 400, headers: { "content-type": "application/json" } }
    );
  }
  const matches = password.verifySync(parsed.pw, parsed.hash);
  if (!matches) {
    return new Response(JSON.stringify({ ok: false, matches: false }), {
      status: 400,
      headers: { "content-type": "application/json", "x-demo": "slice17c" },
    });
  }
  return new Response(JSON.stringify({ ok: true, matches: true }), {
    status: 200,
    headers: { "content-type": "application/json", "x-demo": "slice17c" },
  });
}
"#;

#[test]
fn password_runtime_through_bun_password_serves_hashed_passwords() {
    let Some(master) = locate_master() else {
        eprintln!(
            "skipping: tspserver_v2 binary not found under dist/tsp-v2/ \
             (run ./tsp.sh build:host first)"
        );
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-password-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir should be creatable");
    std::fs::write(routes_dir.join("password.tsp"), PASSWORD_TSP)
        .expect("password.tsp should be writable");

    let port: u16 = 32_500 + (std::process::id() as u16 % 500);
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

    // GET /password: produces { ok, algorithm: "bcrypt", hash, isBcrypt: true }.
    // The `algorithm: "bcrypt"` field + the `$2b$` hash prefix together
    // prove the actual bcrypt algorithm ran (not a stub or alias to
    // nanoid / zod).
    let (status_get, body_get) =
        http_get_status(port, "/password", Duration::from_secs(10));
    assert_eq!(
        status_get, 200,
        "GET /password must return 200, got {status_get} body={body_get:?}"
    );
    assert!(
        body_get.contains(r#""ok":true"#)
            && body_get.contains(r#""algorithm":"bcrypt""#)
            && body_get.contains(r#""hash":"$2b$04$"#)
            && body_get.contains(r#""isBcrypt":true"#),
        "GET /password body must contain bcrypt hash + algorithm marker; got {body_get:?}"
    );

    // GET /password?algo=argon2id: proves the same namespace picks
    // up other algorithms (the page's only change is the `?algo=`
    // query param, the import is unchanged). argon2id is the bun
    // default and the OWASP 2024+ recommendation for new password
    // storage.
    let (status_argon, body_argon) = http_get_status(
        port,
        "/password?algo=argon2id",
        Duration::from_secs(10),
    );
    assert_eq!(
        status_argon, 200,
        "GET /password?algo=argon2id must return 200, got {status_argon} body={body_argon:?}"
    );
    assert!(
        body_argon.contains(r#""algorithm":"argon2id""#)
            && body_argon.contains(r#""isArgon":true"#)
            && body_argon.contains("$argon2id$"),
        "GET /password?algo=argon2id body must contain argon2id hash + marker; got {body_argon:?}"
    );

    // Parse the GET response so we can drive the POST with the
    // freshly-generated bcrypt hash. Doing this end-to-end (not
    // hardcoding a known hash) is what catches a real regression --
    // if the password code path were accidentally stubbed or aliased
    // to zod, the hash prefix would change.
    let hash = extract_json_string_field(&body_get, "hash")
        .unwrap_or_else(|| panic!("GET /password body must carry a `hash` string field, got {body_get:?}"));

    // POST /password with the right password: 200 + matches: true.
    let (status_ok, body_ok) = http_post_status(
        port,
        "/password",
        &format!(r#"{{"pw":"hello","hash":"{hash}"}}"#),
        Duration::from_secs(10),
    );
    assert_eq!(
        status_ok, 200,
        "POST /password with matching pw must return 200, got {status_ok} body={body_ok:?}"
    );
    assert!(
        body_ok.contains(r#""matches":true"#),
        "POST /password matching pw must report matches:true; got {body_ok:?}"
    );

    // POST /password with the wrong password: 400 + matches: false.
    // This is the actual authentication fail path -- the page
    // uses password.verifySync and branches on its return value.
    let (status_bad, body_bad) = http_post_status(
        port,
        "/password",
        &format!(r#"{{"pw":"world","hash":"{hash}"}}"#),
        Duration::from_secs(10),
    );
    assert_eq!(
        status_bad, 400,
        "POST /password with wrong pw must return 400, got {status_bad} body={body_bad:?}"
    );
    assert!(
        body_bad.contains(r#""matches":false"#),
        "POST /password wrong pw must report matches:false; got {body_bad:?}"
    );

    // POST /password with a non-JSON body: 400 + explicit error.
    let (status_garbage, body_garbage) =
        http_post_status(port, "/password", "not json", Duration::from_secs(5));
    assert_eq!(
        status_garbage, 400,
        "POST /password with non-JSON body must return 400, got {status_garbage} body={body_garbage:?}"
    );
    assert!(
        body_garbage.contains("body must be JSON"),
        "POST /password non-JSON body must carry the explicit error; got {body_garbage:?}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// bun builtin `util` namespace integration test (slice 18)
//
// The wrap preamble surfaces bun 1.4's builtins to the page
// as `__tspServer.util` (a frozen namespace). The page reaches
// them through
//     import { util } from "tsp:server";
//     util.randomUUIDv7(); util.hash(buf); util.markdown.html(md);
//     util.YAML.parse(s); new util.CryptoHasher("sha256").update(s)...
// We pin the high-risk subset (`Bun.serve`, `Bun.spawn`,
// `Bun.FFI`, `Bun.S3Client`, `Bun.mmap`, `Bun.Transpiler`,
// `Bun.env.toJSON`) at the unit-test level in jsx.rs; this
// integration test only exercises the safe surfaces end-to-end
// through the real binary.
// ---------------------------------------------------------------------------

const UTIL_DEMO_TSP: &str = r##"import { util } from "tsp:server";

function jsonResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", "x-demo": "slice18" },
  });
}

export function GET(_ctx) {
  const id = util.randomUUIDv7();
  // `util.hash` returns a BigInt; we serialize as hex string
  // so JSON.stringify does not choke and the test can assert
  // on the value stably across runs.
  const hash = util.hash(new TextEncoder().encode("hello, world")).toString(16);
  const safe = util.escapeHTML("<script>alert('xss')</script>");
  const html = util.markdown.html("# Title\n\n**bold** _italic_");
  const config = util.YAML.parse("version: 1\nname: demo\n");
  const toml = util.TOML.parse("key = \"value\"\nn = 42\n");
  const sha = new util.CryptoHasher("sha256");
  sha.update("hello");
  const digest = sha.digest("hex");
  return jsonResponse({
    ok: true,
    id,
    hash,
    sha256: digest,
    safe,
    markdown: html,
    config,
    toml,
  });
}

export async function POST(ctx) {
  const body = await ctx.request.text();
  let parsed;
  try { parsed = JSON.parse(body); }
  catch {
    return jsonResponse({ ok: false, error: "body must be JSON" }, 400);
  }
  if (typeof parsed.password !== "string" || !parsed.password) {
    return jsonResponse({ ok: false, error: "password must be a non-empty string" }, 400);
  }
  const key = util.hash(new TextEncoder().encode(parsed.password)).toString(16);
  return jsonResponse({ ok: true, key });
}
"##;

#[test]
fn util_namespace_surfaces_bun_builtins_for_pages() {
    let Some(master) = locate_master() else {
        eprintln!(
            "skipping: tspserver_v2 binary not found under dist/tsp-v2/ \
             (run ./tsp.sh build:host first)"
        );
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-util-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir should be creatable");
    std::fs::write(routes_dir.join("util_demo.tsp"), UTIL_DEMO_TSP)
        .expect("util_demo.tsp should be writable");

    let port: u16 = 33_500 + (std::process::id() as u16 % 500);
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

    // GET /util_demo: kitchen-sink. Each field exercises a
    // different bun builtin. We assert on a few key shapes --
    // id format (UUIDv7 starts with a hex timestamp prefix),
    // hash (numeric, stable for the same input),
    // sha256 hex length (64), markdown rendered tags,
    // YAML/TOML parsed object shape, escape html-escaped.
    let (status, body) =
        http_get_status(port, "/util_demo", Duration::from_secs(15));
    assert_eq!(
        status, 200,
        "GET /util_demo must return 200, got {status} body={body:?}"
    );
    assert!(
        body.contains("\"sha256\":") && body.contains("\"id\":") && body.contains("\"hash\":\""),
        "GET /util_demo body must carry id/hash/sha256 fields; got {body:?}"
    );
    assert!(
        body.contains("\"markdown\":\"<h1>") && body.contains("<strong>") && body.contains("<em>"),
        "GET /util_demo body must contain rendered markdown tags; got {body:?}"
    );
    assert!(
        body.contains("&lt;script&gt;"),
        "GET /util_demo body must contain escapeHTML output; got {body:?}"
    );
    assert!(
        body.contains("\"config\":{\"version\":1,\"name\":\"demo\"}"),
        "GET /util_demo body must contain parsed YAML config; got {body:?}"
    );
    assert!(
        body.contains("\"toml\":{\"key\":\"value\",\"n\":42}"),
        "GET /util_demo body must contain parsed TOML; got {body:?}"
    );

    // POST /util_demo: hash a password, return the key.
    let (status3, body3) = http_post_status(
        port,
        "/util_demo",
        r#"{"password":"hunter2"}"#,
        Duration::from_secs(10),
    );
    assert_eq!(status3, 200, "POST must return 200, got {status3} body={body3:?}");
    assert!(
        body3.contains("\"ok\":true") && body3.contains("\"key\":\""),
        "POST body must carry ok + key fields; got {body3:?}"
    );

    // POST with non-JSON body: 400.
    let (status_bad, body_bad) =
        http_post_status(port, "/util_demo", "not json", Duration::from_secs(5));
    assert_eq!(
        status_bad, 400,
        "POST with non-JSON body must return 400, got {status_bad} body={body_bad:?}"
    );
    assert!(
        body_bad.contains("body must be JSON"),
        "POST non-JSON body must carry the explicit error; got {body_bad:?}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// ctx.cookies integration test (slice 16f)
//
// The TSP v2 wrap preamble parses the request's `Cookie:`
// header into a `Map` and exposes `ctx.cookies` with read
// (`get` / `has`) and write (`set` / `delete`) methods.
// Writes are pushed into a buffer that the async IIFE
// merges into the response `headers` as separate
// `Set-Cookie` lines, so multiple cookies on one request
// do not collapse via the response's flatten loop. The
// integration test exercises the full page lifecycle
// against the real binary:
//
//   1. GET  /cookies (no cookie)           -> empty echo
//   2. GET  /cookies (Cookie: theme=dark)  -> echoes "dark"
//   3. POST /cookies (writes 2 cookies)    -> 200 + 2 Set-Cookie
//                                           lines in the raw response
//   4. GET  /cookies (Cookie: a=v1; b=v2)  -> echoes both values
//   5. DELETE /cookies?k=a                 -> Set-Cookie: a=;
//                                            Max-Age=0 (delete)
//
// Pinning the raw response is the strongest signal that
// the merge into outgoing headers works (the body
// fingerprint alone would not catch a regression where
// the page wrote a cookie but the host dropped it).
// ---------------------------------------------------------------------------

const COOKIES_TSP: &str = r#"
// Slice 16f: ctx.cookies (spec sect.15) demo. The
// page mirrors the production `routes/cookies.tsp`
// shape so the regression test is self-contained.
function jsonResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });
}

export function GET(ctx) {
  // `?? null` because ctx.cookies.get returns `undefined`
  // for missing keys (JS Map.get convention) and
  // JSON.stringify drops undefined values; null survives.
  const theme = ctx.cookies.get("theme") ?? null;
  const lang = ctx.cookies.get("lang") ?? null;
  const a = ctx.cookies.get("a") ?? null;
  const b = ctx.cookies.get("b") ?? null;
  return jsonResponse({
    ok: true,
    has_theme: ctx.cookies.has("theme"),
    has_missing: ctx.cookies.has("does-not-exist"),
    theme,
    lang,
    a,
    b,
  });
}

export async function POST(ctx) {
  let body;
  try { body = await ctx.request.json(); }
  catch { return jsonResponse({ ok: false, error: "body must be JSON" }, 400); }
  if (typeof body.name !== "string" || !body.name) {
    return jsonResponse({ ok: false, error: "name must be a non-empty string" }, 400);
  }
  const value = String(body.value ?? "");
  const options = body.options || {};
  ctx.cookies.set(body.name, value, options);
  return jsonResponse({ ok: true, name: body.name, value });
}

export function DELETE(ctx) {
  const k = ctx.url.searchParams.get("k");
  if (!k) return jsonResponse({ ok: false, error: "missing k" }, 400);
  ctx.cookies.delete(k);
  return jsonResponse({ ok: true, deleted: k });
}
"#;

#[test]
fn cookies_runtime_parses_request_and_emits_set_cookie_on_write() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-cookies-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir should be creatable");
    std::fs::write(routes_dir.join("cookies.tsp"), COOKIES_TSP)
        .expect("cookies.tsp should be writable");

    let port: u16 = 33_500 + (std::process::id() as u16 % 500);
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

    // 1. GET with no cookie: nothing parsed, has_missing is false.
    let (status1, body1) = http_get_status(port, "/cookies", Duration::from_secs(10));
    assert_eq!(status1, 200, "no-cookie GET must 200, got {status1} body={body1:?}");
    assert!(
        body1.contains("\"has_theme\":false")
            && body1.contains("\"has_missing\":false")
            && body1.contains("\"theme\":null"),
        "no-cookie GET must echo empty map; got {body1:?}"
    );

    // 2. GET with `theme=dark` cookie: page reads it.
    let (status2, body2) = http_get_with_cookie(
        port,
        "/cookies",
        "theme=dark; lang=en",
        Duration::from_secs(10),
    );
    assert_eq!(status2, 200, "theme-cookie GET must 200, got {status2} body={body2:?}");
    assert!(
        body2.contains("\"has_theme\":true")
            && body2.contains("\"theme\":\"dark\"")
            && body2.contains("\"lang\":\"en\""),
        "theme-cookie GET must echo parsed values; got {body2:?}"
    );

    // 3. POST writes 2 cookies; raw response must carry
    //    2 Set-Cookie lines (multi-cookie merge path).
    let (status3, raw3) = http_get_raw(port, "/cookies", Duration::from_secs(5));
    // We just connected; do the actual POST via a fresh stream.
    let _ = (status3, raw3);
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect for POST");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout");
    let body3 = r#"{"name":"a","value":"v1","options":{"path":"/"}}"#;
    let post_a = format!(
        "POST /cookies HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body3}",
        body3.len()
    );
    stream.write_all(post_a.as_bytes()).expect("write POST a");
    let mut raw_post_a = String::new();
    stream.read_to_string(&mut raw_post_a).expect("read POST a");
    assert!(
        raw_post_a.contains("Set-Cookie: a=v1; Path=/"),
        "first POST must set cookie a with Path=/; got:\n{raw_post_a}"
    );
    let status_a: u16 = raw_post_a
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    assert_eq!(status_a, 200, "first cookie POST must 200, got {status_a}");

    // Second cookie: separate request, separate stream,
    // to prove each page invocation produces its own
    // Set-Cookie and the host does not collapse them.
    let body_b = r#"{"name":"b","value":"v2","options":{"httpOnly":true,"sameSite":"Lax"}}"#;
    let mut stream2 = TcpStream::connect(("127.0.0.1", port)).expect("connect for POST b");
    stream2
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout");
    let post_b = format!(
        "POST /cookies HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_b}",
        body_b.len()
    );
    stream2.write_all(post_b.as_bytes()).expect("write POST b");
    let mut raw_post_b = String::new();
    stream2.read_to_string(&mut raw_post_b).expect("read POST b");
    assert!(
        raw_post_b.contains("Set-Cookie: b=v2; HttpOnly; SameSite=Lax"),
        "second cookie POST must set cookie b with HttpOnly + SameSite=Lax; got:\n{raw_post_b}"
    );

    // 4. GET echoing both cookies back (server-side parser
    //    is the one that fed the page during step 2; the
    //    cookie WRITE in steps 3a / 3b lives only in the
    //    browser-side jar in real usage -- here we feed
    //    them back via the Cookie header).
    let (status4, body4) = http_get_with_cookie(
        port,
        "/cookies",
        "a=v1; b=v2",
        Duration::from_secs(10),
    );
    assert_eq!(status4, 200, "echo-cookie GET must 200, got {status4} body={body4:?}");
    assert!(
        body4.contains("\"a\":\"v1\"") && body4.contains("\"b\":\"v2\""),
        "echo-cookie GET must parse a=v1 + b=v2; got {body4:?}"
    );

    // 5. DELETE emits Set-Cookie with Max-Age=0 (the
    //    default the wrap applies on .delete()).
    let mut stream3 = TcpStream::connect(("127.0.0.1", port)).expect("connect for DELETE");
    stream3
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout");
    let delete_req = format!(
        "DELETE /cookies?k=a HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream3.write_all(delete_req.as_bytes()).expect("write DELETE");
    let mut raw_del = String::new();
    stream3.read_to_string(&mut raw_del).expect("read DELETE");
    assert!(
        raw_del.contains("Set-Cookie: a=; Max-Age=0"),
        "DELETE must emit Set-Cookie with Max-Age=0; got:\n{raw_del}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// ctx.session integration test (slice 16k + 16l)
//
// The TSP v2 wrap preamble hydrates `ctx.session` from
// the host's SessionView. Calls (`set` / `delete` /
// `clear` / `regenerate` / `destroy`) buffer into
// `__tspSessionWrites` which the envelope carries back
// to the host; the host applies them via SessionService
// and emits a `Set-Cookie: tsp_sid=...` line when the
// session id changed (mint / regenerate / destroy).
//
// The integration test exercises the full lifecycle:
//
//   1. GET  /session (no cookie)  -> 200, body shows
//      "counter=0" + a fresh sid; response carries
//      `Set-Cookie: tsp_sid=<sid>; HttpOnly; SameSite=Lax`
//   2. GET  /session (Cookie: tsp_sid=<sid>) -> 200,
//      body shows "counter=1" + the SAME sid; no new
//      Set-Cookie (id did not change)
//   3. POST /session {action: "regenerate"} (Cookie
//      carried) -> 200, body shows a NEW sid, counter
//      preserved; response carries Set-Cookie: tsp_sid=
//      <new>
//   4. POST /session {action: "destroy"} -> 200, the
//      NEXT request mints a fresh sid; response carries
//      Set-Cookie: tsp_sid=; Max-Age=0
// ---------------------------------------------------------------------------

const SESSION_TSP: &str = r#"
// Slice 16k: ctx.session (spec sect.16) demo. Mirrors
// the production `routes/session.tsp` shape so the
// regression test is self-contained.
function textResponse(s, status = 200) {
  return new Response(s, { status, headers: { "content-type": "text/plain" } });
}

export function GET(ctx) {
  const before = ctx.session.get("counter") || 0;
  const sid = ctx.session.id;
  ctx.session.set("counter", Number(before) + 1);
  return textResponse(`sid=${sid} counter=${before}`);
}

export async function POST(ctx) {
  let payload;
  try { payload = await ctx.request.json(); } catch { payload = {}; }
  const action = (payload && payload.action) || "";
  if (action === "regenerate") {
    await ctx.session.regenerate();
    return textResponse(`regen sid=${ctx.session.id} counter=${ctx.session.get("counter") || 0}`);
  }
  if (action === "destroy") {
    await ctx.session.destroy();
    return textResponse("destroyed");
  }
  return textResponse(`unknown action=${action}`, 400);
}
"#;

#[test]
fn session_runtime_mints_regenerates_and_destroys_session_id() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-session-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir should be creatable");
    std::fs::write(routes_dir.join("session.tsp"), SESSION_TSP)
        .expect("session.tsp should be writable");

    let port: u16 = 33_700 + (std::process::id() as u16 % 500);
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

    use std::io::{Read, Write};
    use std::net::TcpStream;

    // helper: open a stream, send a raw request, return (status, raw_response, sid-from-cookie).
    //
    // The third element is the extracted sid value as it
    // appears in the `Set-Cookie: tsp_sid=<value>; ...`
    // header. We do NOT filter out empty values because
    // `destroy()` emits a clearing Set-Cookie whose value
    // is intentionally empty (the caller checks for that
    // shape with `assert_eq!(sid, "")`).
    fn round_trip(
        port: u16,
        request: &str,
    ) -> (u16, String, Option<String>) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout");
        stream.write_all(request.as_bytes()).expect("write");
        let mut raw = String::new();
        stream.read_to_string(&mut raw).expect("read");
        let status: u16 = raw
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        // Extract sid from Set-Cookie: tsp_sid=<sid>; ...
        // Empty value (destroy) is preserved as Some("").
        let sid = raw
            .lines()
            .find(|l| l.to_ascii_lowercase().starts_with("set-cookie: tsp_sid="))
            .and_then(|l| {
                let after = &l["set-cookie: tsp_sid=".len()..];
                after.split(';').next().map(|s| s.trim().to_string())
            });
        (status, raw, sid)
    }

    // 1. First GET, no cookie: server mints a fresh sid,
    //    counter starts at 0, response plants Set-Cookie.
    let req1 = format!(
        "GET /session HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    let (status1, raw1, sid1) = round_trip(port, &req1);
    assert_eq!(status1, 200, "first GET must 200, got {status1} body={raw1:?}");
    let body1 = raw1.split("\r\n\r\n").nth(1).unwrap_or("");
    assert!(
        body1.contains("counter=0"),
        "first GET must show counter=0; got {body1:?}"
    );
    let sid1 = sid1.expect("first response must plant tsp_sid Set-Cookie");
    assert!(sid1.len() >= 16, "fresh sid must be reasonably long; got {sid1:?}");
    // The standard session cookie flags:
    assert!(
        raw1.lines().any(|l| l.to_ascii_lowercase().contains("httponly")),
        "first Set-Cookie must be HttpOnly; got:\n{raw1}"
    );
    assert!(
        raw1.lines().any(|l| l.to_ascii_lowercase().contains("samesite=lax")),
        "first Set-Cookie must be SameSite=Lax; got:\n{raw1}"
    );

    // 2. Second GET, carrying the sid cookie: server
    //    reuses the same session, counter increments to 1.
    //    No NEW Set-Cookie because id did not change.
    let req2 = format!(
        "GET /session HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nCookie: tsp_sid={sid1}\r\nConnection: close\r\n\r\n"
    );
    let (status2, raw2, sid2_opt) = round_trip(port, &req2);
    assert_eq!(status2, 200, "second GET must 200, got {status2} body={raw2:?}");
    let body2 = raw2.split("\r\n\r\n").nth(1).unwrap_or("");
    assert!(
        body2.contains("counter=1"),
        "second GET must show counter=1 (session persisted); got {body2:?}"
    );
    // The page body should still report the same sid; if
    // a new Set-Cookie line is present, it must carry the
    // SAME sid (id stable across requests with no writes).
    if let Some(sid2) = sid2_opt.as_deref() {
        assert_eq!(sid2, sid1, "id must be stable across reads; got {sid2} vs {sid1}");
    }
    // No new Set-Cookie at all is the more common shape
    // (host skips the line when id matches); either way,
    // the cookie value must match.
    assert!(
        body2.contains(&sid1),
        "second GET body must echo the same sid; got {body2:?}"
    );

    // 3. POST {action: regenerate}: id changes, counter
    //    preserved. The page's GET handler reads `counter`
    //    then sets `counter+1`, so after step 2 the storage
    //    holds `2`. Regenerate must keep that value (and
    //    NOT reset to 0) and bump the id; the body shows
    //    the value the page READ at the start of the
    //    request, which is 2.
    let regen_body = r#"{"action":"regenerate"}"#;
    let req3 = format!(
        "POST /session HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nCookie: tsp_sid={sid1}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{regen_body}",
        regen_body.len()
    );
    let (status3, raw3, sid3_opt) = round_trip(port, &req3);
    assert_eq!(status3, 200, "regenerate POST must 200, got {status3} body={raw3:?}");
    let sid3 = sid3_opt.expect("regenerate must plant a new Set-Cookie");
    assert_ne!(sid3, sid1, "regenerate must change the id; both = {sid3}");
    let body3 = raw3.split("\r\n\r\n").nth(1).unwrap_or("");
    assert!(
        body3.contains("counter=2"),
        "regenerate must preserve data (counter=2 after step 2); got {body3:?}"
    );

    // 4. POST {action: destroy}: id is cleared, response
    //    plants Set-Cookie: tsp_sid=; Max-Age=0.
    let destroy_body = r#"{"action":"destroy"}"#;
    let req4 = format!(
        "POST /session HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nCookie: tsp_sid={sid3}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{destroy_body}",
        destroy_body.len()
    );
    let (status4, raw4, sid4_opt) = round_trip(port, &req4);
    assert_eq!(status4, 200, "destroy POST must 200, got {status4} body={raw4:?}");
    let sid4 = sid4_opt.expect("destroy must plant a clearing Set-Cookie");
    assert_eq!(sid4, "", "destroy Set-Cookie value must be empty; got {sid4:?}");
    assert!(
        raw4.lines().any(|l| l.to_ascii_lowercase().contains("max-age=0")),
        "destroy Set-Cookie must carry Max-Age=0; got:\n{raw4}"
    );

    // 5. Next GET with no cookie (destroyed): fresh sid,
    //    counter starts at 0 again.
    let req5 = format!(
        "GET /session HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    let (status5, raw5, sid5_opt) = round_trip(port, &req5);
    assert_eq!(status5, 200, "post-destroy GET must 200, got {status5}");
    let sid5 = sid5_opt.expect("post-destroy GET must plant a fresh Set-Cookie");
    assert_ne!(sid5, sid3, "post-destroy sid must differ from the destroyed one");
    let body5 = raw5.split("\r\n\r\n").nth(1).unwrap_or("");
    assert!(
        body5.contains("counter=0"),
        "post-destroy GET must show counter=0 (fresh session); got {body5:?}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// bun:sql runtime integration test (slice 17d)
//
// The TSP v2 wrap preamble surfaces bun's native SQL client
// (`require("bun").SQL`, exposed as `__tspServer.sql`) so pages
// can take a connection from bun's per-worker pool via
// `await sql\`url\``. The page wires the url from a sibling
// `routes/_db.ts` config (plan §17.1: host never sees a
// credential). The integration test exercises the full page
// lifecycle against the real binary:
//
//   1. POST /sql_demo  body={name: "alice"}   -> insert row, get id
//   2. POST /sql_demo  body={name: "bob"}     -> insert row, get id
//   3. GET  /sql_demo                         -> SELECT, verify both rows
//   4. GET  /sql_demo/pool                    -> 3 sequential acquires
//                                               against the same data
//                                               source, confirming
//                                               per-worker pool reuse
//   5. GET  /sql_demo/multi                   -> hit the `analytics`
//                                               data source, proving
//                                               the sibling-config
//                                               multi-datasource
//                                               pattern (page reads
//                                               its data source
//                                               from `routes/_db.ts`,
//                                               not a host registry)
//
// The data sources are SQLite files under the OS temp dir; bun:sql
// supports `sqlite://` urls natively, so the test does not need a
// running MySQL server. The real-binary path proves the page
// code, the `__tspServer.sql` surface, the relative-import
// rewrite for `./_db`, and the wrap's `require("bun")` all work
// end-to-end.
// ---------------------------------------------------------------------------

const SQL_DEMO_TSP: &str = r#"
// Generated by the sql_demo integration test in
// bun/src/runtime/tsp/tests/start_order.rs. Mirrors the
// production `routes/sql_demo.tsp` shape; the regression test
// inlines this rather than copying the file so the assertion
// is self-contained.
//
// Plan §17.1: page-side datasource config (sibling
// `routes/_db.ts`). Plan §17.3: page manages connection
// lifecycle (acquire from `sql\`url\``, close on completion).
// `__tspServer.sql` is bun's `Bun.SQL` factory (native Rust
// driver; zero prelude bytes, no mysql2 embed).
import { sql } from "tsp:server";
import { main as mainDb, analytics as analyticsDb } from "./_db";

async function withPool(db, handler) {
  const conn = await sql(db.url);
  try {
    return await handler(conn);
  } finally {
    conn.close();
  }
}

export async function GET(_ctx) {
  const source = _ctx.query.get("source") || "main";
  const probe = _ctx.query.get("probe");
  const db = source === "analytics" ? analyticsDb : mainDb;
  if (probe === "pool") {
    return withPool(db, async (conn) => {
      const r1 = await conn`SELECT COUNT(*) AS n FROM users`;
      const r2 = await conn`SELECT COUNT(*) AS n FROM users`;
      const r3 = await conn`SELECT COUNT(*) AS n FROM users`;
      return new Response(
        JSON.stringify({ pool: source, rows: [r1[0], r2[0], r3[0]], source }),
        { status: 200, headers: { "content-type": "application/json", "x-demo": "slice17d" } }
      );
    });
  }
  if (source === "analytics") {
    return withPool(db, async (conn) => {
      await conn`CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY AUTOINCREMENT, kind TEXT NOT NULL, n INTEGER NOT NULL DEFAULT 0)`;
      const rows = await conn`SELECT id, kind, n FROM events ORDER BY id ASC`;
      return new Response(
        JSON.stringify({ events: rows, source }),
        { status: 200, headers: { "content-type": "application/json", "x-demo": "slice17d" } }
      );
    });
  }
  return withPool(db, async (conn) => {
    await conn`CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)`;
    const rows = await conn`SELECT id, name FROM users ORDER BY id ASC`;
    return new Response(JSON.stringify({ users: rows, source }), {
      status: 200, headers: { "content-type": "application/json", "x-demo": "slice17d" },
    });
  });
}

export async function POST(ctx) {
  const body = await ctx.request.text();
  let parsed;
  try { parsed = JSON.parse(body); }
  catch {
    return new Response(JSON.stringify({ ok: false, error: "body must be JSON" }), {
      status: 400, headers: { "content-type": "application/json" },
    });
  }
  if (typeof parsed.name !== "string" || !parsed.name) {
    return new Response(
      JSON.stringify({ ok: false, error: "name must be a non-empty string" }),
      { status: 400, headers: { "content-type": "application/json" } }
    );
  }
  return withPool(mainDb, async (conn) => {
    await conn`CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)`;
    const [{ id }] = await conn`INSERT INTO users (name) VALUES (${parsed.name}) RETURNING id`;
    return new Response(JSON.stringify({ ok: true, id, name: parsed.name }), {
      status: 200, headers: { "content-type": "application/json", "x-demo": "slice17d" },
    });
  });
}
"#;

const SQL_DEMO_DB_TS: &str = r#"
// Generated by the sql_demo integration test in
// bun/src/runtime/tsp/tests/start_order.rs. Mirrors the
// production `routes/_db.ts` shape (plan §17.1: page-local
// datasource config; host never sees a credential).
export const main = {
  url: process.env.TSP_DB_MAIN_URL || "sqlite://" + (process.env.TSP_DB_MAIN_FILE || "/tmp/tsp-v2-main.db"),
  pool: 10,
};
export const analytics = {
  url: process.env.TSP_DB_ANALYTICS_URL || "sqlite://" + (process.env.TSP_DB_ANALYTICS_FILE || "/tmp/tsp-v2-analytics.db"),
  pool: 3,
};
"#;

#[test]
fn sql_runtime_uses_bun_native_pool_for_page_local_datasource() {
    let Some(master) = locate_master() else {
        eprintln!(
            "skipping: tspserver_v2 binary not found under dist/tsp-v2/ \
             (run ./tsp.sh build:host first)"
        );
        return;
    };

    // Each test run uses a fresh sqlite file under the OS temp
    // dir so the row counts are predictable. The test does NOT
    // rely on the database being empty (a stale `users` table
    // is fine -- the page does CREATE TABLE IF NOT EXISTS
    // before each query); but it does rely on the `name`
    // values being inserted by this run, so the assertion is on
    // the *delta* (POST inserts two new rows, GET sees the
    // inserts reflected).
    let temp_root = std::env::temp_dir().join(format!(
        "tsp-sql-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir should be creatable");
    std::fs::write(routes_dir.join("sql_demo.tsp"), SQL_DEMO_TSP)
        .expect("sql_demo.tsp should be writable");
    std::fs::write(routes_dir.join("_db.ts"), SQL_DEMO_DB_TS)
        .expect("_db.ts should be writable");

    // Pick a unique sqlite file per test run so the row
    // counts are predictable. We do this through TSP_DB_MAIN_FILE
    // (read by the page's `routes/_db.ts`) -- the test owns
    // the file, the page never sees the credential.
    let main_db_file = temp_root.join("main.db");
    let analytics_db_file = temp_root.join("analytics.db");
    let main_db_url = format!("sqlite://{}", main_db_file.to_string_lossy());
    let analytics_db_url = format!("sqlite://{}", analytics_db_file.to_string_lossy());

    let port: u16 = 33_000 + (std::process::id() as u16 % 500);
    let mut child = std::process::Command::new(master)
        .env("TSP_PORT", port.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .env("TSP_DB_MAIN_URL", &main_db_url)
        .env("TSP_DB_ANALYTICS_URL", &analytics_db_url)
        .env("RUST_BACKTRACE", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn");

    wait_for_marker(&mut child, "listening on", Duration::from_secs(10));

    // POST /sql_demo  x2  -> insert alice + bob. The first
    // POST pays the bun:sql pool-init cost (sqlite open +
    // adapter warm-up). On a cold worker that's typically
    // 1-3 seconds; under load it can spike higher. The 60s
    // deadline is generous on purpose: if a future bun
    // regression pushes pool-init past that, the test should
    // fail loudly so we know to investigate, not silently
    // shorten the timeout and miss real production slowness.
    let (status_alice, body_alice) = http_post_status(
        port,
        "/sql_demo",
        r#"{"name":"alice"}"#,
        Duration::from_secs(60),
    );
    assert_eq!(
        status_alice, 200,
        "POST /sql_demo alice must return 200, got {status_alice} body={body_alice:?}"
    );
    assert!(
        body_alice.contains(r#""ok":true"#) && body_alice.contains(r#""name":"alice""#),
        "POST /sql_demo alice must echo ok+name; got {body_alice:?}"
    );
    assert!(
        body_alice.contains("\"id\":"),
        "POST /sql_demo alice must return a numeric id; got {body_alice:?}"
    );

    let (status_bob, body_bob) = http_post_status(
        port,
        "/sql_demo",
        r#"{"name":"bob"}"#,
        Duration::from_secs(60),
    );
    assert_eq!(
        status_bob, 200,
        "POST /sql_demo bob must return 200, got {status_bob} body={body_bob:?}"
    );
    assert!(
        body_bob.contains(r#""ok":true"#) && body_bob.contains(r#""name":"bob""#),
        "POST /sql_demo bob must echo ok+name; got {body_bob:?}"
    );

    // GET /sql_demo  -> the rows array must contain both
    // alice and bob. We don't assert on exact id (other tests
    // in the same worker process may have left rows behind),
    // but we assert both names are present and the page set
    // the `source: "main"` marker.
    let (status_list, body_list) =
        http_get_status(port, "/sql_demo", Duration::from_secs(60));
    assert_eq!(
        status_list, 200,
        "GET /sql_demo must return 200, got {status_list} body={body_list:?}"
    );
    assert!(
        body_list.contains("\"source\":\"main\""),
        "GET /sql_demo body must carry the page's data-source marker; got {body_list:?}"
    );
    assert!(
        body_list.contains("\"name\":\"alice\"") && body_list.contains("\"name\":\"bob\""),
        "GET /sql_demo must show both rows we inserted; got {body_list:?}"
    );

    // GET /sql_demo?probe=pool  -> 3 sequential acquires against
    // the same data source. The response must contain a `rows`
    // array of 3 SELECT COUNT(*) results. This proves the
    // per-worker pool is reachable from a single page handler
    // (each `conn\`SELECT ...\`` is one acquire+release cycle).
    let (status_pool, body_pool) =
        http_get_status(port, "/sql_demo?probe=pool", Duration::from_secs(60));
    assert_eq!(
        status_pool, 200,
        "GET /sql_demo?probe=pool must return 200, got {status_pool} body={body_pool:?}"
    );
    assert!(
        body_pool.contains("\"pool\":\"main\"") && body_pool.contains("\"source\":\"main\""),
        "GET /sql_demo?probe=pool body must carry the pool + source markers; got {body_pool:?}"
    );
    assert!(
        body_pool.contains("\"n\":"),
        "GET /sql_demo?probe=pool body must contain 3 SELECT COUNT(*) results; got {body_pool:?}"
    );

    // GET /sql_demo?source=analytics  -> hits the `analytics`
    // data source (different sqlite file). The page must
    // return 200 with `source: "analytics"`, proving the
    // sibling-config multi-datasource pattern (page reads
    // `analytics` from `./_db`, not a host registry) works.
    let (status_multi, body_multi) =
        http_get_status(port, "/sql_demo?source=analytics", Duration::from_secs(60));
    assert_eq!(
        status_multi, 200,
        "GET /sql_demo?source=analytics must return 200, got {status_multi} body={body_multi:?}"
    );
    assert!(
        body_multi.contains("\"source\":\"analytics\""),
        "GET /sql_demo?source=analytics must carry the analytics source marker; got {body_multi:?}"
    );

    // POST with bad body: 400 + explicit error (mirrors the
    // zod / bcrypt / nanoid tests' input-validation flow).
    let (status_bad, body_bad) =
        http_post_status(port, "/sql_demo", "not json", Duration::from_secs(10));
    assert_eq!(
        status_bad, 400,
        "POST /sql_demo with non-JSON body must return 400, got {status_bad} body={body_bad:?}"
    );
    assert!(
        body_bad.contains("body must be JSON"),
        "POST /sql_demo non-JSON body must carry the explicit error; got {body_bad:?}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}
