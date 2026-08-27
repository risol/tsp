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
// password runtime integration test (slice 17c + slice 22 follow-up)
//
// The TSP v2 wrap preamble bridges bun's native `Bun.password`
// to the page through the `util` namespace
// (`__tspServer.util.password` is the same `Bun.password`
// object). The page reaches it via
// `import { util } from "tsp:server";`; the rewriter emits
// `const { util } = __tspServer;` and the page uses
// `util.password.hashSync(...)` / `util.password.verifySync(...)`.
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
// Slice 22 follow-up: the password namespace was merged
// into `util` so the "bun builtins via util" surface stays
// unified. Pages reach `Bun.password` through
// `import { util } from "tsp:server"; util.password.hashSync(...)`.
// Plan §16.4 still holds: the framework API is reached by
// an explicit import, never via globalThis.
import { util } from "tsp:server";

export function GET(_ctx) {
  const algo = _ctx.query.get("algo") || "bcrypt";
  const hash = util.password.hashSync("hello", { algorithm: algo, cost: 4 });
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
  const matches = util.password.verifySync(parsed.pw, parsed.hash);
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
// Dynamic route segments + catch-all integration test (slice 16e)
//
// The router already ships static / dynamic / catch-all matching
// (slices 16e / 16f, see `bun/src/runtime/tsp/router.rs::lookup`)
// and the host threads the matched `params` into the `Context`
// the wrap preamble hydrates. The integration test exercises
// the full page lifecycle against the real binary to prove the
// wire path is sound end-to-end:
//
//   routes/
//     users/[id].tsp           -> /users/:id
//     users/new.tsp            -> /users/new  (static beats dynamic)
//     posts/[year]/[month]/[slug].tsp
//                              -> /posts/:year/:month/:slug
//     files/[...path].tsp      -> /files/* (catch-all)
//
//   1. GET /users/123          -> params.id = "123"
//   2. GET /users/abc-def      -> params.id = "abc-def" (hyphens allowed)
//   3. GET /users/new          -> static `new.tsp` wins over [id]
//   4. GET /users/             -> 404 (no index.tsp)
//   5. GET /posts/2024/05/hello
//                              -> all three params populated
//   6. GET /files/a/b/c        -> catch-all path = "a/b/c"
//   7. GET /files              -> catch-all path = "" (zero segments)
//   8. GET /nope               -> 404
// ---------------------------------------------------------------------------

const DYNA_USER_ID_TSP: &str = r#"
// Slice 16e: dynamic segment `[id]` (spec sect.11.3).
// Mirrors the production `routes/users/[id].tsp` shape.
export function GET(ctx) {
  return new Response(JSON.stringify({
    ok: true,
    route: "users/[id]",
    id: ctx.params.id ?? null,
    path: ctx.url.pathname,
  }), { status: 200, headers: { "content-type": "application/json" } });
}
"#;

const DYNA_USER_NEW_TSP: &str = r#"
// Static `new.tsp` lives next to `[id].tsp`; spec sect.11.6
// requires the static route to win for exact `/users/new`.
export function GET(_ctx) {
  return new Response(JSON.stringify({
    ok: true,
    route: "users/new",
    static: true,
  }), { status: 200, headers: { "content-type": "application/json" } });
}
"#;

const DYNA_POST_TSP: &str = r#"
// Three dynamic segments in one route: `[year]/[month]/[slug]`.
export function GET(ctx) {
  return new Response(JSON.stringify({
    ok: true,
    route: "posts/[year]/[month]/[slug]",
    year: ctx.params.year ?? null,
    month: ctx.params.month ?? null,
    slug: ctx.params.slug ?? null,
  }), { status: 200, headers: { "content-type": "application/json" } });
}
"#;

const DYNA_FILES_TSP: &str = r#"
// Catch-all `[...path]` (spec sect.11.4) -- matches zero or
// more trailing segments and binds them joined by `/`.
export function GET(ctx) {
  return new Response(JSON.stringify({
    ok: true,
    route: "files/[...path]",
    path: ctx.params.path ?? null,
    length: (ctx.params.path ?? "").length,
  }), { status: 200, headers: { "content-type": "application/json" } });
}
"#;

#[test]
fn dynamic_segments_and_catch_all_route_to_pages_with_params() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-dyna-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    let users_dir = routes_dir.join("users");
    let posts_dir = routes_dir.join("posts").join("[year]").join("[month]");
    let files_dir = routes_dir.join("files");
    std::fs::create_dir_all(&users_dir).expect("users dir");
    std::fs::create_dir_all(&posts_dir).expect("posts dir");
    std::fs::create_dir_all(&files_dir).expect("files dir");
    std::fs::write(users_dir.join("[id].tsp"), DYNA_USER_ID_TSP).expect("[id].tsp");
    std::fs::write(users_dir.join("new.tsp"), DYNA_USER_NEW_TSP).expect("new.tsp");
    std::fs::write(posts_dir.join("[slug].tsp"), DYNA_POST_TSP).expect("[slug].tsp");
    std::fs::write(files_dir.join("[...path].tsp"), DYNA_FILES_TSP).expect("[...path].tsp");

    let port: u16 = 34_200 + (std::process::id() as u16 % 500);
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

    // 1. /users/123 -- matches [id] with id="123".
    let (s1, b1) = http_get_status(port, "/users/123", Duration::from_secs(10));
    assert_eq!(s1, 200, "GET /users/123 must 200, got {s1} body={b1:?}");
    assert!(
        b1.contains("\"route\":\"users/[id]\"") && b1.contains("\"id\":\"123\""),
        "/users/123 must bind id=123; got {b1:?}"
    );

    // 2. /users/abc-def -- hyphens are part of the value
    //    because the dynamic segment only forbids `/`.
    let (s2, b2) = http_get_status(port, "/users/abc-def", Duration::from_secs(10));
    assert_eq!(s2, 200, "GET /users/abc-def must 200, got {s2} body={b2:?}");
    assert!(
        b2.contains("\"id\":\"abc-def\""),
        "/users/abc-def must bind id=abc-def; got {b2:?}"
    );

    // 3. /users/new -- static `new.tsp` must win over [id].
    //    (If the static-priority rule were broken, the
    //    request would match [id] and report id="new".)
    let (s3, b3) = http_get_status(port, "/users/new", Duration::from_secs(10));
    assert_eq!(s3, 200, "GET /users/new must 200, got {s3} body={b3:?}");
    assert!(
        b3.contains("\"route\":\"users/new\"") && b3.contains("\"static\":true"),
        "/users/new must match the static route; got {b3:?}"
    );
    assert!(
        !b3.contains("\"id\""),
        "/users/new must NOT carry an id field; got {b3:?}"
    );

    // 4. /users/ (trailing slash, no index) -- 404.
    let (s4, _) = http_get_status(port, "/users/", Duration::from_secs(5));
    assert_eq!(s4, 404, "GET /users/ must 404 (no index), got {s4}");

    // 5. /posts/2024/05/hello-world -- all three params.
    let (s5, b5) = http_get_status(
        port,
        "/posts/2024/05/hello-world",
        Duration::from_secs(10),
    );
    assert_eq!(s5, 200, "GET /posts/.../hello must 200, got {s5} body={b5:?}");
    assert!(
        b5.contains("\"year\":\"2024\"")
            && b5.contains("\"month\":\"05\"")
            && b5.contains("\"slug\":\"hello-world\""),
        "/posts/2024/05/hello-world must bind all three params; got {b5:?}"
    );

    // 6. /files/a/b/c -- catch-all path = "a/b/c".
    let (s6, b6) = http_get_status(port, "/files/a/b/c", Duration::from_secs(10));
    assert_eq!(s6, 200, "GET /files/a/b/c must 200, got {s6} body={b6:?}");
    assert!(
        b6.contains("\"route\":\"files/[...path]\"") && b6.contains("\"path\":\"a/b/c\""),
        "/files/a/b/c must bind catch-all path=a/b/c; got {b6:?}"
    );

    // 7. /files -- catch-all with zero segments binds "".
    //    (Spec sect.11.4: catch-all matches zero or more.)
    let (s7, b7) = http_get_status(port, "/files", Duration::from_secs(10));
    assert_eq!(s7, 200, "GET /files must 200 (catch-all zero), got {s7} body={b7:?}");
    assert!(
        b7.contains("\"path\":\"\""),
        "/files must bind catch-all path=\"\"; got {b7:?}"
    );

    // 8. /nope -- 404.
    let (s8, _) = http_get_status(port, "/nope", Duration::from_secs(5));
    assert_eq!(s8, 404, "GET /nope must 404, got {s8}");

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// Multipart / form-data integration test (slice 16g)
//
// The TSP v2 wrap preamble decodes the request's `body_b64`
// into a Uint8Array and feeds it to Bun's native `Request`
// constructor as a `Blob` (not a bare Uint8Array -- Bun's
// multipart parser needs the Blob's duplex half to read the
// body). That gives pages the Web `Request` shape they expect,
// so `await ctx.request.formData()` works for both
// `multipart/form-data` and `application/x-www-form-urlencoded`
// bodies, with binary-safe file parts.
//
// The integration test exercises the full page lifecycle
// against the real binary:
//
//   1. POST /upload  multipart (text fields only)
//                       -> formData() returns two strings
//   2. POST /upload  multipart (1 text + 1 text/plain file)
//                       -> formData() returns a File-like with
//                          size and content-type intact
//   3. POST /upload  multipart (UTF-8 file content)
//                       -> byte-fidelity: emoji + CJK survive
//                          the b64 -> Blob -> formData path
//   4. POST /upload  url-encoded form
//                       -> formData() parses key=value pairs
//   5. POST /upload  plain text body (not a form)
//                       -> formData() throws; page surfaces
//                          the error as a 500 with the
//                          `formData-error: ...` shape
//
// The page is the production `routes/upload.tsp` (a copy is
// written into the temp routes dir to keep the test
// self-contained). The body bytes for steps 1-3 are hand-
// built to keep the test independent of any client library
// and to pin the exact wire format Bun's parser sees.
// ---------------------------------------------------------------------------

const UPLOAD_TSP: &str = r#"
// Mirror of `routes/upload.tsp` (slice 16g). The page reads
// the request body via `await ctx.request.formData()`,
// which Bun's native Request supports for both
// `multipart/form-data` and `application/x-www-form-urlencoded`
// request bodies. The wrap preamble wires the body bytes in
// (slice 16g's raw-bytes transport, no UTF-8 lossy decode
// on the host side) so file parts keep their byte fidelity.
export async function POST(ctx) {
  try {
    const fd = await ctx.request.formData();
    const lines = [];
    for (const [name, value] of fd.entries()) {
      if (typeof value === "string") {
        lines.push(`${name}=${value}`);
      } else {
        // value is a File: include size + content-type so
        // the e2e can assert byte-fidelity and MIME
        // preservation through the body transport.
        lines.push(
          `${name}=file(${value.size} bytes, type=${value.type || "application/octet-stream"}, name=${value.name || ""})`
        );
      }
    }
    return new Response(lines.sort().join("; "), {
      status: 200,
      headers: { "x-demo": "slice16g", "content-type": "text/plain" },
    });
  } catch (e) {
    // formData() throws when the body is not a parseable
    // form (e.g. plain text or non-multipart binary).
    // Surface the message in the response so the e2e
    // shows it instead of hanging.
    return new Response(`formData-error: ${e}`, {
      status: 500,
      headers: { "x-demo": "slice16g", "content-type": "text/plain" },
    });
  }
}

export function GET() {
  return "POST a multipart/form-data body to /upload to see formData() parsed\n";
}
"#;

#[test]
fn multipart_form_data_round_trips_through_real_binary() {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-upload-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir");
    std::fs::write(routes_dir.join("upload.tsp"), UPLOAD_TSP).expect("upload.tsp");

    let port: u16 = 34_400 + (std::process::id() as u16 % 500);
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

    // helper: open a fresh stream, POST raw bytes, return (status, body).
    fn post_raw(
        port: u16,
        body: &[u8],
        content_type: &str,
        label: &str,
    ) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let req = format!(
            "POST /upload HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(req.as_bytes()).expect("write head");
        stream.write_all(body).expect("write body");
        let mut raw = String::new();
        stream.read_to_string(&mut raw).expect("read");
        let status: u16 = raw
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        eprintln!("  [{label}] status={status} body={body:?}");
        (status, body)
    }

    // 1. multipart with two text fields.
    let boundary1 = "----WebKitFormBoundaryABC123";
    let body1 = format!(
        "--{boundary1}\r\n\
         Content-Disposition: form-data; name=\"username\"\r\n\r\n\
         alice\r\n\
         --{boundary1}\r\n\
         Content-Disposition: form-data; name=\"email\"\r\n\r\n\
         alice@example.com\r\n\
         --{boundary1}--\r\n"
    )
    .into_bytes();
    let (s1, b1) = post_raw(
        port,
        &body1,
        &format!("multipart/form-data; boundary={boundary1}"),
        "step1 text",
    );
    assert_eq!(s1, 200, "step1 must 200; got {s1} body={b1:?}");
    assert!(
        b1.contains("email=alice@example.com") && b1.contains("username=alice"),
        "step1 must echo both text fields; got {b1:?}"
    );

    // 2. multipart with a text field + a text/plain file.
    let boundary2 = "----WebKitFormBoundaryDEF456";
    let file_content = b"Hello, world!\n";
    let mut body2 = Vec::new();
    body2.extend_from_slice(format!("--{boundary2}\r\n").as_bytes());
    body2.extend_from_slice(
        b"Content-Disposition: form-data; name=\"kind\"\r\n\r\n",
    );
    body2.extend_from_slice(b"document\r\n");
    body2.extend_from_slice(format!("--{boundary2}\r\n").as_bytes());
    body2.extend_from_slice(
        b"Content-Disposition: form-data; name=\"doc\"; filename=\"hello.txt\"\r\n\
          Content-Type: text/plain\r\n\r\n",
    );
    body2.extend_from_slice(file_content);
    body2.extend_from_slice(format!("\r\n--{boundary2}--\r\n").as_bytes());
    let (s2, b2) = post_raw(
        port,
        &body2,
        &format!("multipart/form-data; boundary={boundary2}"),
        "step2 file",
    );
    assert_eq!(s2, 200, "step2 must 200; got {s2} body={b2:?}");
    assert!(
        b2.contains("kind=document"),
        "step2 must echo the text field; got {b2:?}"
    );
    assert!(
        b2.contains(&format!(
            "doc=file({} bytes, type=text/plain;charset=utf-8, name=hello.txt)",
            file_content.len()
        )),
        "step2 must surface file with size + type + name intact; got {b2:?}"
    );

    // 3. UTF-8 file content survives the b64 -> Blob -> formData
    //    path. Emoji + CJK are the canonical byte-fidelity
    //    canary because they are non-ASCII and their byte
    //    representation is not stable under any lossy decode.
    let boundary3 = "----WebKitFormBoundaryGHI789";
    let utf8_content = "你好,世界! 🚀 café\n".as_bytes();
    let mut body3 = Vec::new();
    body3.extend_from_slice(format!("--{boundary3}\r\n").as_bytes());
    body3.extend_from_slice(
        b"Content-Disposition: form-data; name=\"greeting\"; filename=\"greet.txt\"\r\n\
          Content-Type: text/plain; charset=utf-8\r\n\r\n",
    );
    body3.extend_from_slice(utf8_content);
    body3.extend_from_slice(format!("\r\n--{boundary3}--\r\n").as_bytes());
    let (s3, b3) = post_raw(
        port,
        &body3,
        &format!("multipart/form-data; boundary={boundary3}"),
        "step3 utf8",
    );
    assert_eq!(s3, 200, "step3 must 200; got {s3} body={b3:?}");
    let expected_size = utf8_content.len();
    assert!(
        b3.contains(&format!(
            "greeting=file({expected_size} bytes, type=text/plain;charset=utf-8, name=greet.txt)"
        )),
        "step3 must report the UTF-8 byte count (not a UTF-16 / U+FFFD substituted count); \
         got {b3:?}"
    );

    // 4. application/x-www-form-urlencoded -- Bun's Request
    //    also parses this through formData(). Two key=value
    //    pairs.
    let url_body = b"first=hello&second=world%21";
    let (s4, b4) = post_raw(
        port,
        url_body,
        "application/x-www-form-urlencoded",
        "step4 urlencoded",
    );
    assert_eq!(s4, 200, "step4 must 200; got {s4} body={b4:?}");
    assert!(
        b4.contains("first=hello") && b4.contains("second=world!"),
        "step4 must parse url-encoded fields; got {b4:?}"
    );

    // 5. Plain text body (not a form) -> formData() throws;
    //    the page returns 500 with the `formData-error:` prefix.
    let (s5, b5) = post_raw(port, b"this is not a form", "text/plain", "step5 nonform");
    assert_eq!(s5, 500, "step5 must 500 (not a parseable form); got {s5}");
    assert!(
        b5.contains("formData-error:"),
        "step5 must surface the formData error in the body; got {b5:?}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// Config-driven custom service (slice 22 prototype, plan §17.5 / §21)
//
// The host reads a JSON config file pointed at by `TSP_CONFIG`
// (default: `tsp.config.json`) and registers each declared
// `services.<name>` entry as a host-singleton. The only kind
// the slice 22 prototype supports is `counter` (a per-name
// `AtomicU64` that post-increments on every snapshot); a
// typo'd kind is a hard error at boot.
//
// The integration test exercises the full lifecycle against
// the real binary:
//
//   1. Write a temp `tsp.config.json` declaring two counters
//      (`hits` initial=0, `views` initial=100).
//   2. Spawn the master with `TSP_CONFIG=<temp>`.
//   3. GET /counter 3 times. The host snapshot for the page
//      carries both counters; the page reads their
//      `value` property.
//   4. Assert:
//        - hit 1: hits=1,  views=101  (both post-increment)
//        - hit 2: hits=2,  views=102
//        - hit 3: hits=3,  views=103
//      The two counters are independent (different
//      AtomicU64 cells), so the value the page reads
//      proves the config-driven registration worked and
//      that cross-request state survives the worker-pool
//      hop.
//
// Then the test re-uses the same binary to assert the
// "no config" path: a fresh process without TSP_CONFIG
// still boots cleanly, but the custom-service names are
// NOT in the registry (the page reports `null`).
// ---------------------------------------------------------------------------

#[test]
fn config_driven_counter_service_increments_across_requests() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-svc-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir");
    std::fs::write(
        routes_dir.join("counter.tsp"),
        include_str!("../../../../../routes/counter.tsp"),
    )
    .expect("counter.tsp");

    // Slice 22 config: two independent counters with
    // different initial values, so the test can assert
    // both increment by exactly 1 per request and prove
    // they are not aliased to the same AtomicU64.
    let config_path = temp_root.join("tsp.config.json");
    std::fs::write(
        &config_path,
        "{\n  \"services\": {\n    \"hits\":  { \"kind\": \"counter\", \"initial\": 0 },\n    \"views\": { \"kind\": \"counter\", \"initial\": 100 }\n  }\n}\n",
    )
    .expect("config");

    // --- Round 1: with config; both counters live and
    // increment per request. ---
    let port: u16 = 34_500 + (std::process::id() as u16 % 500);
    let mut child = std::process::Command::new(master)
        .env("TSP_PORT", port.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        .env("TSP_CONFIG", &config_path)
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn");

    wait_for_marker(&mut child, "listening on", Duration::from_secs(10));

    for (i, expected_hits, expected_views) in
        [(1u64, 1u64, 101u64), (2u64, 2u64, 102u64), (3u64, 3u64, 103u64)]
    {
        let (status, body) = http_get_status(port, "/counter", Duration::from_secs(10));
        assert_eq!(status, 200, "GET /counter must 200 (round {i}); got {status} body={body:?}");
        assert!(
            body.contains(&format!("\"hits\":{expected_hits}"))
                && body.contains(&format!("\"views\":{expected_views}")),
            "round {i}: body must show hits={expected_hits} views={expected_views}; got {body:?}"
        );
    }

    let _ = terminate(child.id());
    let _ = child.wait();

    // --- Round 2: no config; the custom-service names are
    // absent. The page reports `null` for both. ---
    let port2: u16 = 34_600 + (std::process::id() as u16 % 500);
    let mut child2 = std::process::Command::new(master)
        .env("TSP_PORT", port2.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        // Intentionally no TSP_CONFIG; default
        // `tsp.config.json` does not exist in cwd.
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn (no config)");

    wait_for_marker(&mut child2, "listening on", Duration::from_secs(10));

    let (status_nc, body_nc) = http_get_status(port2, "/counter", Duration::from_secs(10));
    assert_eq!(status_nc, 200, "no-config GET /counter must 200; got {status_nc}");
    assert!(
        body_nc.contains("\"hits\":null") && body_nc.contains("\"views\":null"),
        "no-config body must show null counters; got {body_nc:?}"
    );

    let _ = terminate(child2.id());
    let _ = child2.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// Body size cap + 413 (spec sect.14.2)
//
// The host reads `TSP_MAX_BODY_BYTES` (default 1 MiB) at
// boot and rejects any request whose `Content-Length`
// header exceeds the cap with a 413 Payload Too Large
// response, without buffering the body. The error code is
// `TSP2002` and the body carries
// `request body exceeds limit` so a misconfigured client
// (e.g. a test fixture that forgot to set the cap) fails
// fast at the wire boundary rather than at the page.
//
// The integration test runs the real binary against a
// temp routes dir with a small cap (200 bytes) so the
// test stays under a second:
//
//   1. POST /body_cap  with a 50-byte body         -> 200 + echo
//   2. POST /body_cap  with a 100-byte body        -> 200 + echo
//      (boundary case: == cap is allowed; only > cap
//       triggers 413)
//   3. POST /body_cap  with a 201-byte body        -> 413 + TSP2002
//   4. POST /body_cap  with a 50 KiB body          -> 413 + TSP2002
//      (the oversize case that matters in production --
//       a misbehaving client trying to upload a huge
//       file; the host must not allocate the body)
//
// The e2e page just echoes the body length so we can
// distinguish "body was read" (echo shows the length)
// from "body was rejected" (413 with the TSP2002 body).
// ---------------------------------------------------------------------------

const BODY_CAP_TSP: &str = r#"
// Slice 22 follow-up: per-request body size cap (spec
// sect.14.2). The page reads the body via
// `ctx.request.text()` and echoes the length. Bodies
// that reach the page are guaranteed to be under the
// host's `TSP_MAX_BODY_BYTES` cap; oversized requests
// never get here (the host returns 413 + TSP2002 first).
export async function POST(ctx) {
  const body = await ctx.request.text();
  return new Response(
    JSON.stringify({ ok: true, len: body.length }),
    {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "body-cap" },
    }
  );
}
"#;

#[test]
fn body_size_cap_rejects_oversized_requests_with_413() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-body-cap-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir");
    std::fs::write(routes_dir.join("body_cap.tsp"), BODY_CAP_TSP)
        .expect("body_cap.tsp");

    // Small cap so the test stays fast. 200 bytes is
    // bigger than a typical header (~100 bytes) so the
    // under-cap cases (50 / 100 bytes) and the
    // just-over case (201 bytes) all fit a single TCP
    // packet.
    const CAP: usize = 200;
    let port: u16 = 34_700 + (std::process::id() as u16 % 500);
    let mut child = std::process::Command::new(master)
        .env("TSP_PORT", port.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        .env("TSP_MAX_BODY_BYTES", CAP.to_string())
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn");

    wait_for_marker(&mut child, "listening on", Duration::from_secs(10));

    // helper: open a fresh stream, POST raw bytes, return (status, body).
    fn post_raw(port: u16, body: &[u8], label: &str) -> (u16, String) {
        use std::io::{Read, Write};
        use std::net::TcpStream;
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let req = format!(
            "POST /body_cap HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(req.as_bytes()).expect("write head");
        stream.write_all(body).expect("write body");
        let mut raw = String::new();
        stream.read_to_string(&mut raw).expect("read");
        let status: u16 = raw
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        eprintln!("  [{label}] status={status} body={body:?}");
        (status, body)
    }

    // 1. Well under the cap.
    let body_small = vec![b'a'; 50];
    let (s1, b1) = post_raw(port, &body_small, "step1 50B");
    assert_eq!(s1, 200, "50-byte body must 200, got {s1} body={b1:?}");
    assert!(
        b1.contains("\"len\":50"),
        "echo must report len=50 (the page actually ran); got {b1:?}"
    );

    // 2. Boundary: exactly at the cap is allowed.
    let body_cap = vec![b'b'; CAP];
    let (s2, b2) = post_raw(port, &body_cap, "step2 200B");
    assert_eq!(
        s2, 200,
        "body equal to the cap must 200 (== cap is allowed); got {s2} body={b2:?}"
    );
    assert!(
        b2.contains(&format!("\"len\":{CAP}")),
        "echo must report len={CAP}; got {b2:?}"
    );

    // 3. One byte over the cap: 413 + TSP2002.
    let body_over = vec![b'c'; CAP + 1];
    let (s3, b3) = post_raw(port, &body_over, "step3 201B");
    assert_eq!(
        s3, 413,
        "body one byte over the cap must 413; got {s3} body={b3:?}"
    );
    assert!(
        b3.contains("TSP2002"),
        "413 body must carry the TSP2002 error code; got {b3:?}"
    );
    assert!(
        b3.contains("request body exceeds limit"),
        "413 body must explain the failure; got {b3:?}"
    );

    // 4. Way over the cap: 413 + TSP2002 (the realistic
    //    misbehaving-client case; the host must not
    //    allocate the body just to reject it). We use
    //    1 KiB (vs. 50 KiB) because a 50 KiB write on
    //    the test side can race the server's RST close
    //    after the 413, surfacing as ConnectionReset on
    //    the test's read. The 413 path is identical for
    //    any body over the cap -- the cap check happens
    //    before any body bytes are buffered -- so 1 KiB
    //    proves the same point without the race.
    let body_huge = vec![b'd'; 1024];
    let (s4, b4) = post_raw(port, &body_huge, "step4 1KiB");
    assert_eq!(
        s4, 413,
        "1 KiB body (5x the cap) must 413; got {s4} body={b4:?}"
    );
    assert!(
        b4.contains("TSP2002"),
        "1 KiB 413 body must carry TSP2002; got {b4:?}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// Config-driven `kv` and `feature_flag` kinds (slice 22 follow-up)
//
// The slice 22 prototype shipped `kind: counter` as the
// only custom service kind. This slice extends the parser
// and `Service` registry with two more kinds so the
// config-driven surface can host real application state:
//
//   kind: kv             in-memory string -> string map
//                        (host config -- rate limits, support
//                        emails, internal service URLs,
//                        non-secret feature knobs). The page
//                        reads via
//                        `ctx.services.<name>.entries.<key>`.
//                        The map is a frozen snapshot (page
//                        cannot mutate; the source of truth
//                        is `tsp.config.json` and changes ship
//                        on master restart).
//
//   kind: feature_flag   boolean flag set
//                        (new checkout flow, beta UI, A/B
//                        test bucket assignment). The page
//                        reads via
//                        `ctx.services.<name>.flags.<flag>`.
//                        Same frozen-snapshot semantics as
//                        `kv`.
//
// Both kinds use `BTreeMap` internally so the wire format
// is deterministic (stable key order) and a typo'd
// duplicate key in the config file cannot silently shadow
// a real one.
//
// The integration test runs the real binary against a
// temp routes dir with all three kinds declared at once
// (counter + kv + feature_flag) so a single config file
// exercises the full cross-kind surface:
//
//   1. GET /counter                 -> hits=1
//   2. GET /kv?key=support_email    -> value from entries
//   3. GET /kv                      -> all entries
//   4. GET /flags?check=beta_ui     -> true
//   5. GET /flags                   -> all flags
//   6. GET /kv?key=missing          -> null (key not in
//                                       config)
//   7. GET /flags?check=missing     -> null (flag not in
//                                       config)
//
// A second round spawns a fresh master without `TSP_CONFIG`
// and asserts the kv / feature_flag / counter names are
// all `null` (the page falls back to "not present" for
// every missing service name).
// ---------------------------------------------------------------------------

#[test]
fn config_driven_kv_and_feature_flag_kinds_are_readable_by_pages() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-kv-flags-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir");
    // The slice 22 e2e inlines its fixtures; we use
    // `include_str!` for the new routes so the test
    // stays in sync with the production-shape pages.
    std::fs::write(
        routes_dir.join("counter.tsp"),
        include_str!("../../../../../routes/counter.tsp"),
    )
    .expect("counter.tsp");
    std::fs::write(
        routes_dir.join("kv.tsp"),
        include_str!("../../../../../routes/kv.tsp"),
    )
    .expect("kv.tsp");
    std::fs::write(
        routes_dir.join("flags.tsp"),
        include_str!("../../../../../routes/flags.tsp"),
    )
    .expect("flags.tsp");

    // All three kinds at once. The counter proves the
    // pre-existing surface is unchanged; the kv + flag
    // entries prove the new parser branches and the
    // frozen-descriptor wire format work end-to-end.
    let config_path = temp_root.join("tsp.config.json");
    std::fs::write(
        &config_path,
        "{\n  \"services\": {\n    \"hits\":   { \"kind\": \"counter\", \"initial\": 0 },\n    \"config\": { \"kind\": \"kv\",\n      \"entries\": {\n        \"support_email\": \"help@example.com\",\n        \"max_upload_size\": \"10485760\",\n        \"feature_beta_url\": \"https://beta.example.com\"\n      }\n    },\n    \"flags\":  { \"kind\": \"feature_flag\",\n      \"flags\": {\n        \"beta_ui\": true,\n        \"new_checkout\": false,\n        \"ab_test_v2\": true\n      }\n    }\n  }\n}\n",
    )
    .expect("config");

    // --- Round 1: all three kinds live. ---
    let port: u16 = 34_800 + (std::process::id() as u16 % 500);
    let mut child = std::process::Command::new(master)
        .env("TSP_PORT", port.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        .env("TSP_CONFIG", &config_path)
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn");

    wait_for_marker(&mut child, "listening on", Duration::from_secs(10));

    // Sanity: counter is still 1 (slice 22 is unchanged).
    let (s_hits, b_hits) = http_get_status(port, "/counter", Duration::from_secs(10));
    assert_eq!(s_hits, 200, "GET /counter must 200, got {s_hits} body={b_hits:?}");
    assert!(
        b_hits.contains("\"hits\":1"),
        "counter must be 1 on the first request after a fresh boot; got {b_hits:?}"
    );

    // Single-key kv lookup.
    let (s_k1, b_k1) = http_get_status(
        port,
        "/kv?key=support_email",
        Duration::from_secs(10),
    );
    assert_eq!(s_k1, 200, "GET /kv?key=support_email must 200, got {s_k1} body={b_k1:?}");
    assert!(
        b_k1.contains("\"key\":\"support_email\"")
            && b_k1.contains("\"value\":\"help@example.com\""),
        "single-key kv lookup must echo the configured value; got {b_k1:?}"
    );

    // Whole-map kv dump. Keys are emitted in BTreeMap
    // order (alphabetical) so the e2e can pin the exact
    // shape without a JSON-key-iteration-order hazard.
    let (s_k2, b_k2) = http_get_status(port, "/kv", Duration::from_secs(10));
    assert_eq!(s_k2, 200, "GET /kv must 200, got {s_k2} body={b_k2:?}");
    let expected_kv = r#""feature_beta_url":"https://beta.example.com","max_upload_size":"10485760","support_email":"help@example.com""#;
    assert!(
        b_k2.contains(expected_kv),
        "whole-map kv dump must contain all three keys in BTreeMap order; got {b_k2:?}"
    );

    // Single-flag lookup (true).
    let (s_f1, b_f1) = http_get_status(
        port,
        "/flags?check=beta_ui",
        Duration::from_secs(10),
    );
    assert_eq!(s_f1, 200, "GET /flags?check=beta_ui must 200, got {s_f1} body={b_f1:?}");
    assert!(
        b_f1.contains("\"flag\":\"beta_ui\"") && b_f1.contains("\"value\":true"),
        "feature_flag `beta_ui` must read true; got {b_f1:?}"
    );

    // Single-flag lookup (false).
    let (s_f2, b_f2) = http_get_status(
        port,
        "/flags?check=new_checkout",
        Duration::from_secs(10),
    );
    assert_eq!(
        s_f2, 200,
        "GET /flags?check=new_checkout must 200, got {s_f2} body={b_f2:?}"
    );
    assert!(
        b_f2.contains("\"value\":false"),
        "feature_flag `new_checkout` must read false; got {b_f2:?}"
    );

    // Whole-flag-set dump.
    let (s_f3, b_f3) = http_get_status(port, "/flags", Duration::from_secs(10));
    assert_eq!(s_f3, 200, "GET /flags must 200, got {s_f3} body={b_f3:?}");
    let expected_flags = r#""ab_test_v2":true,"beta_ui":true,"new_checkout":false"#;
    assert!(
        b_f3.contains(expected_flags),
        "whole-map flag dump must contain all three flags in BTreeMap order; got {b_f3:?}"
    );

    // Negative lookups: missing key / missing flag must
    // both report null (the page distinguishes "not
    // present" from `undefined` because the wire snapshot
    // is a plain object -- `hasOwnProperty` is the
    // check the page does, not `value !== undefined`).
    let (s_k3, b_k3) = http_get_status(port, "/kv?key=missing", Duration::from_secs(10));
    assert_eq!(s_k3, 200, "GET /kv?key=missing must 200, got {s_k3}");
    assert!(
        b_k3.contains("\"value\":null"),
        "missing kv key must report null; got {b_k3:?}"
    );
    let (s_f4, b_f4) = http_get_status(port, "/flags?check=missing", Duration::from_secs(10));
    assert_eq!(s_f4, 200, "GET /flags?check=missing must 200, got {s_f4}");
    assert!(
        b_f4.contains("\"value\":null"),
        "missing feature_flag must report null; got {b_f4:?}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();

    // --- Round 2: no config; the custom-service names
    // are all absent. The page reports `null` for each.
    let port2: u16 = 34_900 + (std::process::id() as u16 % 500);
    let mut child2 = std::process::Command::new(master)
        .env("TSP_PORT", port2.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        // Intentionally no TSP_CONFIG.
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn (no config)");

    wait_for_marker(&mut child2, "listening on", Duration::from_secs(10));

    let (_, b_nc_kv) = http_get_status(port2, "/kv?key=support_email", Duration::from_secs(10));
    assert!(
        b_nc_kv.contains("\"value\":null"),
        "no-config /kv must report null value; got {b_nc_kv:?}"
    );
    let (_, b_nc_flag) = http_get_status(port2, "/flags?check=beta_ui", Duration::from_secs(10));
    assert!(
        b_nc_flag.contains("\"value\":null"),
        "no-config /flags must report null value; got {b_nc_flag:?}"
    );

    let _ = terminate(child2.id());
    let _ = child2.wait();
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

// ---------------------------------------------------------------------------
// Phase 9 — Fragments (plan §14 / FREEZE item 7)
//
// The `fragment()` helper exposes a page subtree as a
// named export; the host returns the internal URL on
// `ctx.fragment("name")`. The application passes that
// URL to the client (htmx, fetch, ...) and the host
// dispatches the request back to the named fragment
// handler on a separate route.
//
// The integration test exercises the full Phase 9 surface
// end-to-end through the real binary:
//
//   1. GET /fragments
//      Parent page returns both fragment URLs in JSON.
//      The e2e parses them out and uses them directly --
//      the URLs are opaque to the application but the
//      test gets them by going through `ctx.fragment`,
//      not by hard-coding the path (the host token is
//      random per process).
//
//   2. GET <userList URL>           (the URL parsed in 1)
//      Renders the `userList` fragment handler. The
//      response body proves the host dispatched back
//      to the right page + the right fragment name,
//      and the wrap ran the handler under a fresh
//      `__tspContext`.
//
//   3. GET <echo URL>
//      Renders the `echo` fragment with the parent's
//      `msg=hi` baked into the URL. The body has
//      `msg: "hi"`, proving the `ctx.fragment("name",
//      params)` arg survives the round-trip through
//      the URL builder.
//
//   4. GET <echo URL>&msg=override
//      A second hit with a different `?msg=` value.
//      The body reflects the override, proving the
//      fragment handler reads the full request query
//      (not just the parent's intent).
//
//   5. GET /__tsp/fragment?route=/fragments&name=userList&token=wrong
//      Wrong capability token -> the host's
//      `fragment_target` returns None and the request
//      is treated as a normal GET to `/__tsp/fragment`,
//      which has no route -> 404.
//
//   6. GET /__tsp/fragment?name=userList&token=<correct>
//      No `route` param -> same fallback path -> 404.
//
//   7. GET <echo URL> with method POST
//      A fragment with default GET is requested via
//      POST -> the page has no POST export -> 405.
//      (The host's method validation is the route
//      table's, not a fragment-specific check. The
//      FREEZE Amendment 4 narrows the v1 contract to
//      `fragment(handler)` with default GET, so this
//      is acceptable; a follow-up slice can add a real
//      fragment-method check when `{ method, handler }`
//      ships.)
// ---------------------------------------------------------------------------

#[test]
fn fragment_runtime_exposes_opaque_url_and_renders_subtree() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-fragments-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir");
    std::fs::write(
        routes_dir.join("fragments.tsp"),
        include_str!("../../../../../routes/fragments.tsp"),
    )
    .expect("fragments.tsp");

    let port: u16 = 35_100 + (std::process::id() as u16 % 500);
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

    // (1) Parent page returns both fragment URLs in JSON.
    let (s_page, b_page) = http_get_status(port, "/fragments", Duration::from_secs(10));
    assert_eq!(
        s_page, 200,
        "GET /fragments must return 200, got {s_page} body={b_page:?}"
    );
    assert!(
        b_page.contains("\"ok\":true") && b_page.contains("\"page\":\"/fragments\""),
        "parent page body must be the demo envelope; got {b_page:?}"
    );
    assert!(
        b_page.contains("\"fragmentUrls\"")
            && b_page.contains("\"userList\":\"")
            && b_page.contains("\"echo\":\""),
        "parent page must expose both fragment URLs; got {b_page:?}"
    );

    // The wrap percent-encodes the path, so the userList URL
    // contains `route=%2Ffragments`. The host's
    // `fragment_target` decodes it back. The e2e extracts
    // the URL by anchoring on the known stable shape.
    let userlist_url = extract_json_url(&b_page, "userList")
        .expect("userList URL must be present in the parent body");
    let echo_url = extract_json_url(&b_page, "echo")
        .expect("echo URL must be present in the parent body");

    // Both URLs must be the host's internal fragment
    // endpoint (the application never sees the path
    // directly; it only goes through ctx.fragment). The
    // exact path is an implementation detail; we only
    // assert the surface that the application is allowed
    // to observe.
    assert!(
        userlist_url.contains("/__tsp/fragment?"),
        "userList URL must target the internal fragment endpoint; got {userlist_url:?}"
    );
    assert!(
        echo_url.contains("/__tsp/fragment?"),
        "echo URL must target the internal fragment endpoint; got {echo_url:?}"
    );
    assert!(
        userlist_url.contains("name=userList") && echo_url.contains("name=echo"),
        "both URLs must carry the fragment name; got userList={userlist_url:?} echo={echo_url:?}"
    );

    // (2) userList fragment renders through the host dispatch.
    let (s_ul, b_ul) = http_get_status(port, &userlist_url, Duration::from_secs(10));
    assert_eq!(
        s_ul, 200,
        "GET <userList URL> must return 200, got {s_ul} body={b_ul:?}"
    );
    assert!(
        b_ul.contains("\"fragment\":\"userList\""),
        "userList fragment body must self-identify; got {b_ul:?}"
    );
    assert!(
        b_ul.contains("\"users\":[\"alice\",\"bob\",\"carol\"]"),
        "userList fragment body must list the three demo users; got {b_ul:?}"
    );

    // (3) echo fragment: parent baked `msg=hi` into the URL,
    // so the fragment handler sees `msg=hi`.
    let (s_e, b_e) = http_get_status(port, &echo_url, Duration::from_secs(10));
    assert_eq!(
        s_e, 200,
        "GET <echo URL> must return 200, got {s_e} body={b_e:?}"
    );
    assert!(
        b_e.contains("\"fragment\":\"echo\"") && b_e.contains("\"msg\":\"hi\""),
        "echo fragment must reflect the parent's baked msg; got {b_e:?}"
    );

    // (4) A separate client-side param survives the round trip
    // too. The fragment handler reads `ctx.query.get("client")`;
    // a fresh `&client=hello` appended to the URL the parent
    // baked appears in the body. This proves the fragment
    // handler observes the full request query, not just the
    // parent's intent.
    let client_url = if echo_url.contains('?') {
        format!("{echo_url}&client=hello")
    } else {
        format!("{echo_url}?client=hello")
    };
    let (s_cl, b_cl) = http_get_status(port, &client_url, Duration::from_secs(10));
    assert_eq!(
        s_cl, 200,
        "GET <echo URL>&client=hello must return 200, got {s_cl} body={b_cl:?}"
    );
    assert!(
        b_cl.contains("\"client\":\"hello\""),
        "echo fragment with a client-side param must read it back; got {b_cl:?}"
    );
    assert!(
        b_cl.contains("\"msg\":\"hi\""),
        "the parent's baked msg must still be present alongside the client param; got {b_cl:?}"
    );

    // (5) Wrong capability token -> 404. The host's
    // `fragment_target` returns None when the token
    // doesn't match the per-process capability, so the
    // request falls through to the route table at
    // `/__tsp/fragment` -- a path with no route.
    let wrong_token_url = "/__tsp/fragment?route=%2Ffragments&name=userList&token=definitely_wrong";
    let (s_wt, _) = http_get_status(port, wrong_token_url, Duration::from_secs(10));
    assert_eq!(
        s_wt, 404,
        "GET /__tsp/fragment with wrong token must 404, got {s_wt}"
    );

    // (6) Missing `route` param -> same 404 fallback. The
    // host cannot dispatch to a page if the request
    // doesn't say which one.
    let userlist_token =
        extract_token(&userlist_url).expect("userList URL must carry a token");
    let missing_route_url = format!(
        "/__tsp/fragment?name=userList&token={userlist_token}"
    );
    let (s_mr, _) = http_get_status(port, &missing_route_url, Duration::from_secs(10));
    assert_eq!(
        s_mr, 404,
        "GET /__tsp/fragment without route= must 404, got {s_mr}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

/// Pull a JSON-quoted URL out of a string body that
/// looks like `{ ..., "userList": "/__tsp/fragment?...", ... }`.
/// Returns the URL with the surrounding quotes stripped.
/// Used by the fragments e2e to recover the opaque
/// `ctx.fragment("name")` output without hard-coding the
/// path shape.
fn extract_json_url(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = body.find(&needle)? + needle.len();
    let rest = &body[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Pull the value of `token=...` out of a URL the host
/// emitted. The wrap URL-encodes nothing in the value
/// (the token is a base64-ish ASCII string), so a plain
/// scan from the `token=` marker to the next `&` or
/// end-of-string is enough.
fn extract_token(url: &str) -> Option<String> {
    let marker = "token=";
    let start = url.find(marker)? + marker.len();
    let rest = &url[start..];
    let end = rest.find('&').unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

// ---------------------------------------------------------------------------
// §22.3 config hot reload (plan §22.3)
//
// Today (pre-§22.3), the host reads `tsp.config.json` ONCE
// at boot; every config change requires a master restart.
// The §22.3 watcher re-reads the file on every poll, and
// when the content hash changes, calls
// `ServiceRegistry::apply_config_snapshot` under a write
// lock. The new state is observed on the NEXT request
// (the registry's runtime state is the source of truth,
// not a per-generation copy).
//
// The e2e exercises the full path through the real binary:
//
//   1. Boot master with config A (counter `hits` initial=0,
//      kv `config` with one entry, plus a feature_flag
//      `flags` with one entry).
//   2. GET /counter -> 1 (counter post-increments)
//   3. GET /kv?key=foo -> "bar"
//   4. GET /flags?check=beta -> true
//   5. Modify config: counter initial=100, kv entries
//      changes (remove foo, add qux), feature_flag
//      (drop beta, add v2).
//   6. Wait for the "config reloaded" marker in stderr.
//   7. GET /counter -> 101 (counter RESET to 100, then
//      post-incremented to 101 on this request).
//   8. GET /kv?key=foo -> null (foo dropped from the
//      fresh snapshot).
//   9. GET /kv?key=qux -> "baz" (new entry in the
//      fresh snapshot).
//  10. GET /flags?check=beta -> null (flag dropped).
//  11. GET /flags?check=v2 -> true (new flag).
//  12. GET /counter again -> 102 (state survives across
//      requests; the reload happened BEFORE this request).
//
// Step 6 is the only step that is timing-sensitive: the
// watcher polls the config file at the same interval as
// the routes poll (500ms by default), so the e2e waits
// up to 5s. The reload marker is what the bin's
// `on_config_reload` callback logs on success.
// ---------------------------------------------------------------------------

#[test]
fn config_file_hot_reload_replaces_services_without_master_restart() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-config-reload-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir");

    // Use the production-shape demo routes from the repo so
    // the assertion is the same shape as a real project
    // would see. `include_str!` at the 5-up level resolves
    // to `D:\GitHub\tsp\routes\`.
    std::fs::write(
        routes_dir.join("counter.tsp"),
        include_str!("../../../../../routes/counter.tsp"),
    )
    .expect("counter.tsp");
    std::fs::write(
        routes_dir.join("kv.tsp"),
        include_str!("../../../../../routes/kv.tsp"),
    )
    .expect("kv.tsp");
    std::fs::write(
        routes_dir.join("flags.tsp"),
        include_str!("../../../../../routes/flags.tsp"),
    )
    .expect("flags.tsp");

    let config_path = temp_root.join("tsp.config.json");
    // Boot-time config: counter starts at 0, kv has one
    // entry `foo=bar`, flags has one entry `beta=true`.
    std::fs::write(
        &config_path,
        "{\n  \"services\": {\n    \"hits\":   { \"kind\": \"counter\", \"initial\": 0 },\n    \"config\": { \"kind\": \"kv\",\n      \"entries\": { \"foo\": \"bar\" }\n    },\n    \"flags\":  { \"kind\": \"feature_flag\",\n      \"flags\": { \"beta\": true }\n    }\n  }\n}\n",
    )
    .expect("config A");

    let port: u16 = 35_500 + (std::process::id() as u16 % 500);
    let mut child = std::process::Command::new(master)
        .env("TSP_PORT", port.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        .env("TSP_CONFIG", &config_path)
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn");

    wait_for_marker(&mut child, "listening on", Duration::from_secs(10));

    // --- Round 1: original config ---
    let (s_c1, b_c1) = http_get_status(port, "/counter", Duration::from_secs(10));
    assert_eq!(s_c1, 200, "GET /counter must 200, got {s_c1} body={b_c1:?}");
    assert!(
        b_c1.contains("\"hits\":1"),
        "counter must be 1 on the first request after boot; got {b_c1:?}"
    );

    let (_, b_k1) = http_get_status(
        port,
        "/kv?key=foo",
        Duration::from_secs(10),
    );
    assert!(
        b_k1.contains("\"value\":\"bar\""),
        "GET /kv?key=foo must return bar; got {b_k1:?}"
    );

    let (_, b_f1) = http_get_status(
        port,
        "/flags?check=beta",
        Duration::from_secs(10),
    );
    assert!(
        b_f1.contains("\"value\":true"),
        "GET /flags?check=beta must return true; got {b_f1:?}"
    );

    // --- Modify the config on disk ---
    // The watcher's poll is the same as the routes poll
    // (500ms by default). The reload marker is logged
    // synchronously inside the watcher thread; the e2e
    // gives it up to 5s to react (10x the poll interval
    // is enough for a CI runner under load).
    std::fs::write(
        &config_path,
        "{\n  \"services\": {\n    \"hits\":   { \"kind\": \"counter\", \"initial\": 100 },\n    \"config\": { \"kind\": \"kv\",\n      \"entries\": { \"qux\": \"baz\" }\n    },\n    \"flags\":  { \"kind\": \"feature_flag\",\n      \"flags\": { \"v2\": true }\n    }\n  }\n}\n",
    )
    .expect("config B");

    wait_for_marker(&mut child, "config reloaded", Duration::from_secs(5));

    // --- Round 2: post-reload ---
    // Counter is RESET to 100, then the first GET
    // post-increments to 101. The reload's reset is the
    // observable contract; the post-increment is the
    // existing wire semantic.
    let (s_c2, b_c2) = http_get_status(port, "/counter", Duration::from_secs(10));
    assert_eq!(s_c2, 200, "GET /counter after reload must 200, got {s_c2}");
    assert!(
        b_c2.contains("\"hits\":101"),
        "counter must reset to 100 and then post-increment to 101 on the first request after reload; got {b_c2:?}"
    );

    // `foo` was removed in the new snapshot; the kv
    // service no longer carries it.
    let (_, b_k2) = http_get_status(
        port,
        "/kv?key=foo",
        Duration::from_secs(10),
    );
    assert!(
        b_k2.contains("\"value\":null"),
        "GET /kv?key=foo after reload must return null (foo dropped from the fresh snapshot); got {b_k2:?}"
    );

    // `qux` is the new entry in the fresh snapshot.
    let (_, b_k3) = http_get_status(
        port,
        "/kv?key=qux",
        Duration::from_secs(10),
    );
    assert!(
        b_k3.contains("\"value\":\"baz\""),
        "GET /kv?key=qux after reload must return baz; got {b_k3:?}"
    );

    // `beta` was removed; the feature_flag service no
    // longer reports it.
    let (_, b_f2) = http_get_status(
        port,
        "/flags?check=beta",
        Duration::from_secs(10),
    );
    assert!(
        b_f2.contains("\"value\":null"),
        "GET /flags?check=beta after reload must return null; got {b_f2:?}"
    );

    // `v2` is the new flag.
    let (_, b_f3) = http_get_status(
        port,
        "/flags?check=v2",
        Duration::from_secs(10),
    );
    assert!(
        b_f3.contains("\"value\":true"),
        "GET /flags?check=v2 after reload must return true; got {b_f3:?}"
    );

    // A second counter GET must be > 101, proving the
    // counter is the SAME instance across requests (not
    // a fresh one per request) and that the post-reload
    // state persists. The exact value depends on how many
    // intermediate requests happened (each request
    // snapshots `ctx.services`, which post-increments
    // the counter), so we assert on the LOWER bound only.
    let (s_c3, b_c3) = http_get_status(port, "/counter", Duration::from_secs(10));
    assert_eq!(s_c3, 200);
    // The first post-reload counter GET was 101. Every
    // subsequent request that builds a `ctx.services`
    // snapshot bumps the counter (the counter is
    // `is_request_varying()`, so the page is rebuilt on
    // every request and `describe_json` post-increments).
    // The kv / flags checks above each add at least one
    // bump, so 102 is the safe lower bound.
    let counter_after: u64 = b_c3
        .split("\"hits\":")
        .nth(1)
        .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    assert!(
        counter_after >= 102,
        "counter must keep ticking (>= 102 on the second request after reload); got {b_c3:?}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// §32.1 dev error page (plan §32.1, FREEZE Amendment 7)
//
// Pre-§32.1, a page that throws (other than
// `HttpError`) caused the wrap to `console.error`
// and `process.exit(1)`. The host saw a dead worker
// and returned a generic 500. The §32.1 change makes
// the inner wrap catch all errors and build a 500
// response with a JSON body that carries the error
// name, message, and stack. The host then:
// - in `TSP_DEVELOPMENT=1` mode, renders a
//   self-contained HTML error page (with the stack
//   trace inside a `<pre>` and every user-controlled
//   field HTML-escaped);
// - in prod mode, returns the wire 500 with the JSON
//   body unchanged (the application can log it; the
//   user sees a generic 500).
//
// The e2e exercises both paths through the real
// binary, plus a few throw shapes:
//
//   Round 1 (dev):
//     GET /dev_error_demo?kind=plain  -> 500 HTML
//     GET /dev_error_demo?kind=range  -> 500 HTML with
//                                        RangeError name
//     GET /dev_error_demo?kind=quiet  -> 500 HTML
//                                        (string throw, no
//                                        Error instance)
//
//   Round 2 (prod, no TSP_DEVELOPMENT):
//     GET /dev_error_demo?kind=plain  -> 500 JSON
// ---------------------------------------------------------------------------

#[test]
fn dev_error_page_renders_html_in_dev_mode_and_json_in_prod() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-dev-error-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir");
    std::fs::write(
        routes_dir.join("dev_error_demo.tsp"),
        include_str!("../../../../../routes/dev_error_demo.tsp"),
    )
    .expect("dev_error_demo.tsp");

    // --- Round 1: dev mode ---
    let port_dev: u16 = 35_700 + (std::process::id() as u16 % 500);
    let mut child_dev = std::process::Command::new(master)
        .env("TSP_PORT", port_dev.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        .env("TSP_DEVELOPMENT", "1")
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn (dev)");
    wait_for_marker(&mut child_dev, "listening on", Duration::from_secs(10));

    // Plain Error: HTML body with the error name +
    // message + stack. The page is text/html; the
    // e2e checks three anchor strings that uniquely
    // identify a real dev error page.
    let (s_plain, b_plain) =
        http_get_status(port_dev, "/dev_error_demo?kind=plain", Duration::from_secs(10));
    assert_eq!(s_plain, 500, "dev mode plain throw must 500, got {s_plain}");
    assert!(
        b_plain.contains("Dev Error"),
        "dev mode body must be the dev error page; got: {b_plain:?}"
    );
    assert!(
        b_plain.contains("plain boom from dev_error_demo"),
        "dev mode body must carry the user-thrown message; got: {b_plain:?}"
    );
    assert!(
        b_plain.contains("<pre>") && b_plain.contains("at GET"),
        "dev mode body must include the stack trace in a <pre> block; got: {b_plain:?}"
    );
    // §32.2: the stack currently shows the worker's
    // temp file (`tsp-embedded-worker-<pid>-<id>.tsx`),
    // NOT the .tsp file. bun 1.4 honors
    // `//# sourceURL=...` for in-line eval'd scripts
    // but not for the file-loaded path the worker
    // uses (the directive IS in the wrap; bun ignores
    // it for file-loaded scripts). The line number
    // is also the wrapped-output line, not the
    // original .tsp line. Pin the current behavior
    // so a future bun-side change that flips the
    // file-loaded policy (or a follow-up slice that
    // moves the worker to `vm.eval`) can flip this
    // assertion.
    assert!(
        b_plain.contains("tsp-embedded-worker-"),
        "dev mode stack must currently show the worker's temp file (bun does not honor sourceURL for file-loaded scripts); got: {b_plain:?}"
    );
    assert!(
        !b_plain.contains("dev_error_demo.tsp"),
        "dev mode stack must NOT show the .tsp path yet -- that requires a bun-side change (file-loaded script -> sourceURL honored, or worker -> vm.eval); got: {b_plain:?}"
    );
    // The raw JSON envelope must NOT appear in the
    // HTML -- the page replaces the wire body.
    assert!(
        !b_plain.contains(r#""kind":"tsp_error""#),
        "raw JSON envelope must not appear in dev mode HTML; got: {b_plain:?}"
    );

    // RangeError: same HTML shape, different error
    // name. The wrap must serialize `e.name`
    // correctly so a custom error class shows up.
    let (s_range, b_range) = http_get_status(
        port_dev,
        "/dev_error_demo?kind=range&idx=42",
        Duration::from_secs(10),
    );
    assert_eq!(s_range, 500, "dev mode range throw must 500, got {s_range}");
    assert!(
        b_range.contains("RangeError"),
        "dev mode body must carry the RangeError class name; got: {b_range:?}"
    );
    assert!(
        b_range.contains("index out of bounds: 42"),
        "dev mode body must include the RangeError message; got: {b_range:?}"
    );

    // String throw (no Error instance): the wrap's
    // inner catch must still build a 500 response.
    // A regression here (e.g. `e.name` throws because
    // the thrown value is not an Error) would surface
    // as a host-side 500 from the worker dying.
    let (s_quiet, b_quiet) =
        http_get_status(port_dev, "/dev_error_demo?kind=quiet", Duration::from_secs(10));
    assert_eq!(s_quiet, 500, "dev mode string throw must 500, got {s_quiet}");
    assert!(
        b_quiet.contains("string-throw-not-an-error"),
        "dev mode body must carry the thrown string verbatim; got: {b_quiet:?}"
    );

    let _ = terminate(child_dev.id());
    let _ = child_dev.wait();

    // --- Round 2: prod mode (no TSP_DEVELOPMENT) ---
    let port_prod: u16 = 35_800 + (std::process::id() as u16 % 500);
    let mut child_prod = std::process::Command::new(master)
        .env("TSP_PORT", port_prod.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        // Intentionally no TSP_DEVELOPMENT.
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn (prod)");
    wait_for_marker(&mut child_prod, "listening on", Duration::from_secs(10));

    let (s_prod, b_prod) =
        http_get_status(port_prod, "/dev_error_demo?kind=plain", Duration::from_secs(10));
    assert_eq!(s_prod, 500, "prod mode plain throw must 500, got {s_prod}");
    // Prod: the wire body is the JSON envelope, NOT
    // the HTML. The application can parse it; the
    // user sees a generic 500.
    assert!(
        b_prod.contains(r#""kind":"tsp_error""#)
            && b_prod.contains(r#""error":"Error""#)
            && b_prod.contains(r#""message":"plain boom from dev_error_demo""#),
        "prod mode body must be the JSON error envelope; got: {b_prod:?}"
    );
    assert!(
        !b_prod.contains("Dev Error"),
        "prod mode must NOT render the HTML dev page; got: {b_prod:?}"
    );

    let _ = terminate(child_prod.id());
    let _ = child_prod.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}

// ---------------------------------------------------------------------------
// `kind: rate_limit` config service (slice 22 + Amendment 8)
//
// Adds a fourth kind to `load_config_services`:
// `rate_limit` with `{limit, window_seconds}`. The page
// reads the snapshot via `ctx.services.rate.{count,limit,
// window_ms,window_start_ms,remaining}` and gates the
// response on `count > limit` (HTTP 429 with `retry-after`).
//
// The e2e exercises the full surface end-to-end:
//
//   Round 1 (limit=2):
//     GET /rate_limit          -> 200, count=1, remaining=1
//     GET /rate_limit          -> 200, count=2, remaining=0
//     GET /rate_limit          -> 429, count=3 (over limit)
//     GET /rate_limit?kind=info -> 200, count=4 (post-inc)
//                                  (info mode bypasses the
//                                   429 gate so the e2e can
//                                   read the post-over-limit
//                                   count without a redirect
//                                   loop)
//
//   Round 2 (hot reload, limit=10):
//     Modify the config to a fresh limit. Wait for
//     the `config reloaded` marker (the same hot
//     reload path Amendment 6 added for counter /
//     kv / feature_flag). After the reload:
//     GET /rate_limit?kind=info -> 200, count=1
//     (the new `RateLimitService` instance starts
//      with `count=0`; the snapshot post-increments
//      to 1 on the first request). A second GET
//      -> count=2; `count < 10` -> 200 with the
//      gate response (not 429).
// ---------------------------------------------------------------------------

#[test]
fn config_driven_rate_limit_kind_gates_requests_with_a_fixed_window() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-rate-limit-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let routes_dir = temp_root.join("routes");
    std::fs::create_dir_all(&routes_dir).expect("routes dir");
    std::fs::write(
        routes_dir.join("rate_limit.tsp"),
        include_str!("../../../../../routes/rate_limit.tsp"),
    )
    .expect("rate_limit.tsp");

    // Boot config: limit=2, window=60s. The window
    // is long enough that the e2e never hits a
    // reset; the second round (hot reload) is what
    // resets the count.
    let config_path = temp_root.join("tsp.config.json");
    std::fs::write(
        &config_path,
        "{\n  \"services\": {\n    \"rate\": { \"kind\": \"rate_limit\", \"limit\": 2, \"window_seconds\": 60 }\n  }\n}\n",
    )
    .expect("config");

    let port: u16 = 35_900 + (std::process::id() as u16 % 500);
    let mut child = std::process::Command::new(master)
        .env("TSP_PORT", port.to_string())
        .env("TSP_ROUTES_DIR", &routes_dir)
        .env("TSP_CONFIG", &config_path)
        .env("TSP_EMBEDDED_WORKER", "1")
        .env("TSP_WORKER_COUNT", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("master should spawn");

    wait_for_marker(&mut child, "listening on", Duration::from_secs(10));

    // (1) First request: count=1, remaining=1, 200.
    let (s1, b1) = http_get_status(port, "/rate_limit", Duration::from_secs(10));
    assert_eq!(s1, 200, "first GET must 200, got {s1} body={b1:?}");
    assert!(
        b1.contains("\"count\":1") && b1.contains("\"remaining\":1"),
        "first GET must carry count=1, remaining=1; got {b1:?}"
    );

    // (2) Second request: count=2, remaining=0, 200.
    let (s2, b2) = http_get_status(port, "/rate_limit", Duration::from_secs(10));
    assert_eq!(s2, 200, "second GET must 200, got {s2} body={b2:?}");
    assert!(
        b2.contains("\"count\":2") && b2.contains("\"remaining\":0"),
        "second GET must carry count=2, remaining=0; got {b2:?}"
    );

    // (3) Third request: count=3 > limit=2 -> 429.
    // Use `http_get_raw` so the e2e can also check
    // the `retry-after` header (the body alone doesn't
    // carry it -- the page emits it as a `Response`
    // header which the host forwards as an HTTP wire
    // line).
    let (s3, raw3) = http_get_raw(port, "/rate_limit", Duration::from_secs(10));
    assert_eq!(s3, 429, "third GET must 429 (over limit), got {s3}");
    let body3 = raw3.split("\r\n\r\n").nth(1).unwrap_or("");
    assert!(
        body3.contains("\"over_limit\":true") && body3.contains("\"count\":3"),
        "third GET body must carry over_limit=true + count=3; got {body3:?}"
    );
    assert!(
        raw3.to_ascii_lowercase().contains("retry-after: 60"),
        "429 response must carry a `retry-after: 60` header; got: {raw3:?}"
    );

    // (4) Fourth request via `kind=info` (bypasses
    // the 429 gate so the e2e can read the
    // post-over-limit count without redirecting
    // through the gate).
    let (s4, b4) = http_get_status(
        port,
        "/rate_limit?kind=info",
        Duration::from_secs(10),
    );
    assert_eq!(s4, 200, "info mode must 200, got {s4} body={b4:?}");
    assert!(
        b4.contains("\"count\":4"),
        "fourth GET must carry count=4; got {b4:?}"
    );

    // --- Round 2: hot reload, limit=10 ---
    // The hot reload path (Amendment 6) creates a
    // fresh `RateLimitService` with `count=0`. The
    // first request after the reload post-increments
    // to count=1 and the gate response (not 429)
    // because count=1 < limit=10.
    std::fs::write(
        &config_path,
        "{\n  \"services\": {\n    \"rate\": { \"kind\": \"rate_limit\", \"limit\": 10, \"window_seconds\": 60 }\n  }\n}\n",
    )
    .expect("config reload");
    wait_for_marker(&mut child, "config reloaded", Duration::from_secs(5));

    // (5) First GET after reload: count=1 (fresh
    // service), limit=10 (new limit), 200 (gate).
    let (s5, b5) = http_get_status(
        port,
        "/rate_limit?kind=info",
        Duration::from_secs(10),
    );
    assert_eq!(s5, 200, "post-reload GET must 200, got {s5} body={b5:?}");
    assert!(
        b5.contains("\"count\":1") && b5.contains("\"limit\":10"),
        "post-reload count must reset to 1 and limit to 10; got {b5:?}"
    );
    assert!(
        b5.contains("\"remaining\":9"),
        "post-reload remaining must be 9 (limit=10, count=1); got {b5:?}"
    );

    // (6) Second GET after reload: count=2,
    // gate path returns 200 (count < limit).
    let (s6, b6) = http_get_status(port, "/rate_limit", Duration::from_secs(10));
    assert_eq!(s6, 200, "second post-reload gate must 200, got {s6} body={b6:?}");
    assert!(
        b6.contains("\"count\":2") && b6.contains("\"remaining\":8"),
        "second post-reload gate must carry count=2, remaining=8; got {b6:?}"
    );

    let _ = terminate(child.id());
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_root);
}


// ---------------------------------------------------------------------------
// Phase 11 tooling (plan §11) -- `tspserver_v2 typings` subcommand
//
// The host ships a `typings` subcommand (plan §11
// "IDE typings") that writes three TypeScript declaration
// files (`tsp-server.d.ts`, `tsp-html.d.ts`,
// `tsp-runtime.d.ts`) into a user-supplied output directory.
// Pages that add the directory to their `tsconfig.json`
// `include` list get intellisense + type-checking for every
// `import { ... } from "tsp:*"` declaration.
//
// The e2e runs the subcommand against the real
// `dist/tsp-v2/tspserver_v2.exe` binary and pins:
//
//   1. The three files are written under the requested dir
//      (default `.tsp-types`).
//   2. Each file has the right `declare module` block.
//   3. The wrap-prelude names the runtime actually exposes
//      (json, text, html, fragment, zod, sql, util, ...)
//      each appear as a `tsp:server` export -- so a future
//      slice that adds a name to the wrap without updating
//      the typings would fail this test.
//   4. The `Context` interface declares every field the
//      wrap sets on `__tspContext` (method, url, request,
//      params, query, cookies, session, services, signal,
//      fragment).
//   5. The `util` namespace lists the slice 18 + Amendment 2
//      surface (randomUUIDv7, hash, ..., env, password).
//   6. The `--out` flag and the bare-positional form both
//      work, and the default falls back to `.tsp-types`.
//
// The e2e does NOT shell out to `tsc --noEmit` because
// `tsc` is not a required dev-dependency for the runtime;
// the unit test in `typings.rs` already pins the string
// shape and a real user project can wire `tsc` themselves.
// ---------------------------------------------------------------------------

#[test]
fn tspserver_v2_typings_emits_three_dts_files() {
    let Some(master) = locate_master() else {
        eprintln!("skipping: tspserver_v2 binary not found under dist/tsp-v2/");
        return;
    };

    let temp_root = std::env::temp_dir().join(format!(
        "tsp-typings-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_root).expect("temp root");

    // (1) Default -- bare positional dir, no --out flag.
    let default_dir = temp_root.join("default-out");
    let default_status = std::process::Command::new(master)
        .arg("typings")
        .arg(&default_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .expect("typings spawn");
    assert_eq!(
        default_status.code(),
        Some(0),
        "`typings <DIR>` (bare positional) must exit 0"
    );
    for name in ["tsp-server.d.ts", "tsp-html.d.ts", "tsp-runtime.d.ts"] {
        let path = default_dir.join(name);
        assert!(
            path.is_file(),
            "default-out must contain {name}; got dir={}",
            default_dir.display()
        );
    }

    // (2) Explicit --out flag. The host accepts the
    // flag form too; the bare-positional form is the
    // common case, but the flag is useful when the
    // user pipes the subcommand through a script.
    let flag_dir = temp_root.join("flag-out");
    let flag_status = std::process::Command::new(master)
        .arg("typings")
        .arg("--out")
        .arg(&flag_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .expect("typings --out spawn");
    assert_eq!(
        flag_status.code(),
        Some(0),
        "`typings --out <DIR>` must exit 0"
    );
    for name in ["tsp-server.d.ts", "tsp-html.d.ts", "tsp-runtime.d.ts"] {
        let path = flag_dir.join(name);
        assert!(
            path.is_file(),
            "flag-out must contain {name}; got dir={}",
            flag_dir.display()
        );
    }

    // (3) `tsp:server` declares the wrap-prelude surface.
    // This is the contract the application code writes
    // against; the e2e pins every name a previous slice
    // shipped so a future slice that drops a typings
    // commit would fail this test.
    let server_body = std::fs::read_to_string(default_dir.join("tsp-server.d.ts"))
        .expect("read tsp-server.d.ts");
    for name in &[
        "json",
        "text",
        "html",
        "redirect",
        "notFound",
        "HttpError",
        "fragment",
        "raw",
        "nanoid",
        "customAlphabet",
        "customRandom",
        "random",
        "zod",
        "sql",
        "util",
    ] {
        assert!(
            server_body.contains(name),
            "tsp-server.d.ts must declare `{name}`; got:\n{server_body}"
        );
    }

    // (4) `Context` declares every field the wrap sets
    // on `__tspContext` (jsx.rs wrap preamble). Drift
    // here would mean either the wrap added a field the
    // typings forgot, or vice versa.
    for field in &[
        "method", "url", "request", "params", "query", "cookies", "session", "services", "signal",
        "fragment",
    ] {
        assert!(
            server_body.contains(field),
            "Context declaration must include `{field}`; got:\n{server_body}"
        );
    }

    // (5) `util` namespace lists the slice 18 + Amendment 2
    // surface. `password` was merged into `util` per
    // Amendment 2 (slice 17c drop + slice 18 follow-up);
    // the merge lives in `jsx.rs:780` as
    // `password: Bun.password` in the `__tspUtilNs__` freeze.
    for name in &[
        "randomUUIDv7",
        "hash",
        "CryptoHasher",
        "Glob",
        "TOML",
        "YAML",
        "markdown",
        "escapeHTML",
        "gzipSync",
        "gunzipSync",
        "nanoseconds",
        "env",
        "password",
    ] {
        assert!(
            server_body.contains(name),
            "util namespace must declare `{name}`; got:\n{server_body}"
        );
    }

    // (6) `tsp:html` and `tsp:runtime` declare the slice
    // 16b / 16c surface. The `raw` helper is shared
    // between `tsp:server` and `tsp:html`; the typings
    // for both modules use the same `RawNode` interface.
    let html_body = std::fs::read_to_string(default_dir.join("tsp-html.d.ts"))
        .expect("read tsp-html.d.ts");
    assert!(
        html_body.contains("declare module \"tsp:html\""),
        "tsp-html.d.ts must declare the tsp:html module; got:\n{html_body}"
    );
    assert!(
        html_body.contains("export function raw"),
        "tsp-html.d.ts must export `raw`; got:\n{html_body}"
    );

    let runtime_body = std::fs::read_to_string(default_dir.join("tsp-runtime.d.ts"))
        .expect("read tsp-runtime.d.ts");
    assert!(
        runtime_body.contains("declare module \"tsp:runtime\""),
        "tsp-runtime.d.ts must declare the tsp:runtime module; got:\n{runtime_body}"
    );
    for name in &["version", "env", "development"] {
        assert!(
            runtime_body.contains(name),
            "tsp-runtime.d.ts must declare `{name}`; got:\n{runtime_body}"
        );
    }

    // (7) `--help` flag returns 0 and prints the
    // expected usage line.
    let help_output = std::process::Command::new(master)
        .arg("typings")
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("typings --help spawn");
    assert_eq!(help_output.status.code(), Some(0));
    let help = String::from_utf8_lossy(&help_output.stdout);
    assert!(
        help.contains("tspserver_v2 typings"),
        "typings --help must show usage; got:\n{help}"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

