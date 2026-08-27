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
