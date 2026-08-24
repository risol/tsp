//! TCP listener + minimal HTTP/1.1 responder for TSP v2 PoC 1 slice 2.
//!
//! See `tsp-v2-plan.md` §61 Phase 1. Slice 2 is a hand-rolled, stdlib-only
//! server that:
//!
//! 1. Binds to `0.0.0.0:<port>` (default 3000; override via `TSP_PORT`).
//! 2. Accepts each connection on its own thread.
//! 3. Reads a single request into a fixed-size buffer.
//! 4. Returns a hand-written 404 for every request -- no router, no
//!    `.tsp` execution, no JSC. Slice 3 adds the route scanner; slice 5
//!    adds JSC + transpile.
//!
//! Production HTTP lives behind `bun_uws` (plan §25.3) and arrives when
//! the HTTP path needs async / multi-worker / uWS-grade throughput.
//! Keeping slice 2 stdlib-only means the first compile stays cheap and
//! the bootstrap is auditable line-by-line.
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

/// Default TCP port for the PoC listener.
const DEFAULT_PORT: u16 = 3000;

/// Hand-rolled error type. We deliberately avoid pulling `thiserror` or
/// any other error crate for slice 2 -- one variant per failure mode and
/// `Display` is enough.
#[derive(Debug)]
pub enum HostError {
    /// `TcpListener::bind` failed (port in use, permission denied, etc.).
    Bind(io::Error),
    /// A per-connection `accept` returned an error other than a closed
    /// listener (interrupted system call, EMFILE, etc.).
    Accept(io::Error),
    /// A connection handler failed. Logged with `eprintln!` and the loop
    /// continues -- a single bad client must not take the server down.
    Connection(io::Error),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bind(e) => write!(f, "bind failed: {e}"),
            Self::Accept(e) => write!(f, "accept failed: {e}"),
            Self::Connection(e) => write!(f, "connection handler failed: {e}"),
        }
    }
}

impl std::error::Error for HostError {}

/// Global "stop the accept loop" flag, flipped to `true` by
/// [`serve_until_stopped`] when it observes a non-recoverable accept
/// error or when the process receives SIGINT (the binary's signal
/// handler is the only writer besides the accept loop itself).
static STOP: AtomicBool = AtomicBool::new(false);

/// Bind to `host:port`, accept connections forever, hand each one to a
/// fresh thread running [`handle_connection`]. Returns
/// [`HostError::Bind`] if the listener cannot be created; any error on
/// the accept loop is forwarded as [`HostError::Accept`].
///
/// This is the function the `tspserver_v2` binary calls. Slice 2 -- no
/// graceful shutdown yet, no signal handling. `Ctrl-C` in the terminal
/// terminates the process via the default SIGINT handler; future slices
/// will install a `ctrlc` hook that flips [`STOP`] and drains in-flight
/// connections.
pub fn serve(host: &str, port: u16) -> Result<(), HostError> {
    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr).map_err(HostError::Bind)?;
    eprintln!("TSPv2PoC1: listening on http://{addr} (slice 2 of plan §70 PoC 1)");

    while !STOP.load(Ordering::Acquire) {
        let (stream, peer) = match listener.accept() {
            Ok(pair) => pair,
            Err(e) => {
                // EWOULDBLOCK under non-blocking mode, or interrupted
                // syscall: loop. Anything else is fatal for the listener.
                if e.kind() == io::ErrorKind::Interrupted
                    || e.kind() == io::ErrorKind::WouldBlock
                {
                    continue;
                }
                return Err(HostError::Accept(e));
            }
        };
        eprintln!("TSPv2PoC1: accepted {peer}");
        thread::spawn(move || {
            if let Err(e) = handle_connection(stream) {
                eprintln!("TSPv2PoC1: {e}");
            }
        });
    }
    Ok(())
}

/// Read one request, write one 404 response, close the connection. The
/// response body is the slice-2 marker so a manual `curl` immediately
/// shows whether the listener is up and which slice is serving.
fn handle_connection(mut stream: TcpStream) -> Result<(), HostError> {
    // 8 KiB is comfortably more than any sane request line + headers for
    // a 404 probe; PoC 1 never accepts bodies.
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).map_err(HostError::Connection)?;
    let request = std::str::from_utf8(&buf[..n]).unwrap_or("");
    let path = parse_request_line(request).unwrap_or("/");

    let body = format!(
        "TSP v2 PoC 1 slice 2: route scanner not wired yet (path = {path})\n"
    );
    let head = format!(
        "HTTP/1.1 404 Not Found\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    );

    stream
        .write_all(head.as_bytes())
        .map_err(HostError::Connection)?;
    stream
        .write_all(body.as_bytes())
        .map_err(HostError::Connection)?;
    // `shutdown(Both)` is what makes the client see EOF; without it a
    // half-dozen TCP stacks will keep the connection in CLOSE_WAIT.
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

/// Pull the request-target out of the first line of a `GET /foo HTTP/1.1`
/// style request. Returns `None` for malformed input -- callers fall
/// back to `/` so a probe never crashes the listener.
fn parse_request_line(request: &str) -> Option<&str> {
    let first_line = request.lines().next()?;
    let mut parts = first_line.split_whitespace();
    let _method = parts.next()?;
    let path = parts.next()?;
    // Strip the query string so log lines and future route matchers
    // don't carry `?foo=bar` noise.
    let end = path.find('?').unwrap_or(path.len());
    Some(&path[..end])
}

/// Resolve the listen port for slice 2. `TSP_PORT` env var wins, falling
/// back to [`DEFAULT_PORT`]. Parse failure is a hard error so a typo
/// in the env var does not silently bind to the default.
pub fn resolve_port() -> Result<u16, HostError> {
    match std::env::var("TSP_PORT") {
        Err(_) => Ok(DEFAULT_PORT),
        Ok(s) => s.parse::<u16>().map_err(|_| HostError::Bind(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("TSP_PORT is not a u16: {s:?}"),
        ))),
    }
}
