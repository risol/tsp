//! Slice 16l: Session backends (memory / Redis).
//!
//! Spec sect.16.2 says session state MUST survive page
//! generation reloads; the 16k in-memory store satisfies
//! that for a single host process. 16l adds a Redis
//! backend so multiple host processes (workers /
//! production boxes) can share session state without
//! sticky routing -- the cookie's `tsp_sid` is the key
//! the client always carries, the store is whatever the
//! boot-time env var points at.
//!
//! Both backends implement the same [`SessionBackend`]
//! trait; the host-side `SessionService` (in `services.rs`)
//! only sees the trait, so adding a third backend later
//! (memcached / SQLite / etc.) does not touch the host.
//!
//! Wire model: the backends keep the same `SessionData`
//! / `SessionView` / `SessionWrite` shapes 16k defined.
//! Memory stores them in-process; Redis serialises the
//! data map to a JSON string under `tsp:session:<sid>`.
//! The `id` field is the cookie value AND the storage
//! key, so the page-side `regenerate` call is a
//! rename-or-copy under the hood.
//!
//! The Redis client is hand-rolled to keep the slice
//! "no new dep" discipline (plan sect.25.3). RESP2 is
//! tiny (one byte tag + a CRLF per frame); the surface
//! the backend needs is GET, SETEX (with TTL), DEL,
//! PEXPIRE. No pipelining, no cluster, no streams --
//! a real production switch to `redis-rs` is a later
//! slice if the hand-rolled client becomes a bottleneck.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::services::{SessionData, SessionValue, SessionView, SessionWrite};

/// Common surface every session backend implements.
pub trait SessionBackend: Send + Sync {
    /// Stable name for diagnostics
    /// (`memory` / `redis`). Embedded in the
    /// `ctx.services.session` descriptor snapshot.
    fn name(&self) -> &'static str;
    /// `true` when the backend is wired and accepting
    /// operations. A Redis backend whose TCP connect
    /// failed at boot returns `false` from here so the
    /// host can log the misconfiguration without taking
    /// the whole binary down.
    fn is_available(&self) -> bool;
    /// Look up an existing session by sid. `None` when
    /// the id is unknown or has been destroyed (spec
    /// 16.4 makes a destroyed session no longer usable).
    fn lookup(&self, sid: &str) -> Option<SessionView>;
    /// Mint a fresh id and insert an empty row. The
    /// returned view's `id` is the cookie the response
    /// will plant.
    fn create(&self) -> SessionView;
    /// Apply the page's writes to the session identified
    /// by `current_sid`. Returns the NEW id the host
    /// will plant as `Set-Cookie` (or empty when the
    /// session was destroyed).
    fn apply_writes(&self, current_sid: &str, writes: &[SessionWrite]) -> String;
    /// Total live sessions (memory backend only; the
    /// Redis backend returns 0 -- `DBSIZE` is not in
    /// the minimal hand-rolled surface).
    fn len(&self) -> usize {
        0
    }
}

// =====================================================================
// In-memory backend (16k's store, factored into a backend)
// =====================================================================

/// The in-memory backend. Spec 16.2: this survives page
/// generation reloads because `SessionService` is a
/// runtime-scoped service that lives in the host-owned
/// registry; the backend is only ever dropped when the
/// host shuts down.
pub struct MemoryBackend {
    store: Mutex<HashMap<String, SessionData>>,
    order: Mutex<Vec<String>>,
    cap: usize,
    next_counter: AtomicU64,
}

impl MemoryBackend {
    pub fn new(cap: usize) -> Self {
        assert!(cap > 0, "MemoryBackend cap must be > 0");
        MemoryBackend {
            store: Mutex::new(HashMap::new()),
            order: Mutex::new(Vec::new()),
            cap,
            next_counter: AtomicU64::new(0),
        }
    }

    fn mint_sid(&self) -> String {
        let n = self.next_counter.fetch_add(1, Ordering::Relaxed);
        let high = (n >> 32) as u32;
        let low = n as u32;
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&high.to_be_bytes());
        buf[4..8].copy_from_slice(&low.to_be_bytes());
        let mix = high ^ low;
        for slot in buf[8..16].chunks_mut(4) {
            let v = mix.wrapping_mul(0x9E37_79B9).wrapping_add(n as u32);
            slot.copy_from_slice(&v.to_be_bytes());
        }
        let mut out = String::with_capacity(32);
        use std::fmt::Write as _;
        for byte in buf.iter() {
            let _ = write!(out, "{byte:02x}");
        }
        out
    }

    fn push_order(&self, sid: &str) {
        let mut order = self.order.lock().unwrap();
        if let Some(pos) = order.iter().position(|s| s == sid) {
            order.remove(pos);
        }
        order.push(sid.to_string());
    }

    fn enforce_cap(&self) {
        let mut store = self.store.lock().unwrap();
        let mut order = self.order.lock().unwrap();
        while store.len() > self.cap {
            if let Some(victim) = order.first().cloned() {
                order.remove(0);
                store.remove(&victim);
            } else {
                break;
            }
        }
    }
}

impl SessionBackend for MemoryBackend {
    fn name(&self) -> &'static str {
        "memory"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn lookup(&self, sid: &str) -> Option<SessionView> {
        let store = self.store.lock().unwrap();
        store.get(sid).map(|d| SessionView {
            id: d.id.clone(),
            data: d.data.clone(),
        })
    }

    fn create(&self) -> SessionView {
        let id = self.mint_sid();
        {
            let mut store = self.store.lock().unwrap();
            store.insert(
                id.clone(),
                SessionData {
                    id: id.clone(),
                    data: BTreeMap::new(),
                },
            );
        }
        self.push_order(&id);
        self.enforce_cap();
        SessionView {
            id,
            data: BTreeMap::new(),
        }
    }

    fn apply_writes(&self, current_sid: &str, writes: &[SessionWrite]) -> String {
        if writes.is_empty() {
            return current_sid.to_string();
        }
        let mut new_sid = current_sid.to_string();
        for w in writes {
            match w {
                SessionWrite::Set(name, value) => {
                    if let Some(sid) = non_empty(&new_sid) {
                        let mut store = self.store.lock().unwrap();
                        if let Some(row) = store.get_mut(sid) {
                            row.data.insert(name.clone(), value.clone());
                        } else {
                            eprintln!(
                                "TSPv2PoC1: session write to unknown sid dropped (key={name})"
                            );
                        }
                    }
                }
                SessionWrite::Delete(name) => {
                    if let Some(sid) = non_empty(&new_sid) {
                        let mut store = self.store.lock().unwrap();
                        if let Some(row) = store.get_mut(sid) {
                            row.data.remove(name);
                        }
                    }
                }
                SessionWrite::Clear => {
                    if let Some(sid) = non_empty(&new_sid) {
                        let mut store = self.store.lock().unwrap();
                        if let Some(row) = store.get_mut(sid) {
                            row.data.clear();
                        }
                    }
                }
                SessionWrite::Regenerate => {
                    if let Some(sid) = non_empty(&new_sid) {
                        let moved = {
                            let mut store = self.store.lock().unwrap();
                            store.remove(sid).map(|row| row.data)
                        };
                        if let Some(data) = moved {
                            let fresh = self.mint_sid();
                            self.store.lock().unwrap().insert(
                                fresh.clone(),
                                SessionData {
                                    id: fresh.clone(),
                                    data,
                                },
                            );
                            {
                                let mut order = self.order.lock().unwrap();
                                if let Some(pos) = order.iter().position(|s| s == &sid) {
                                    order.remove(pos);
                                }
                                order.push(fresh.clone());
                            }
                            new_sid = fresh;
                        }
                    }
                }
                SessionWrite::Destroy => {
                    if let Some(sid) = non_empty(&new_sid) {
                        let mut store = self.store.lock().unwrap();
                        store.remove(sid);
                        let mut order = self.order.lock().unwrap();
                        if let Some(pos) = order.iter().position(|s| s == &sid) {
                            order.remove(pos);
                        }
                    }
                    new_sid.clear();
                }
            }
        }
        new_sid
    }

    fn len(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

fn non_empty(s: &str) -> Option<&str> {
    if s.is_empty() { None } else { Some(s) }
}

// =====================================================================
// Redis backend (hand-rolled RESP client)
// =====================================================================

/// Default TTL a freshly created session carries in
/// Redis. 24h matches the typical cookie-max-age; pages
/// that need a different policy can call `regenerate`
/// or we can extend the backend to read a config knob
/// later.
pub const REDIS_SESSION_TTL_SECS_DEFAULT: u64 = 24 * 60 * 60;

/// Key prefix so a Redis shared with other apps does
/// not collide.
pub const REDIS_KEY_PREFIX: &str = "tsp:session:";

/// Minimal Redis backend. The trait implementation
/// translates `lookup` -> `GET`, `create` -> `SET` with
/// the TTL, `apply_writes` -> a sequence of
/// `HSET`/`HDEL`/`DEL` plus an `EXPIRE` refresh, all
/// over a single `TcpStream`. The `Mutex<TcpStream>`
/// serialises commands; 16l is single-process per
/// backend, so a real production switch to a connection
/// pool is a later slice.
///
/// `available` flips to `false` when the initial PING
/// fails so the host can log a misconfiguration without
/// taking the whole binary down. Subsequent commands
/// still try, so a transient outage self-heals once
/// Redis comes back; the next successful command
/// re-flips the flag.
pub struct RedisBackend {
    /// `redis://host:port[/db]`
    url: String,
    /// Parsed `host:port` ready to dial.
    endpoint: String,
    /// Optional db number (`/0`, `/1`, ...).
    db: i64,
    /// Per-session TTL in seconds. 0 means "no
    /// expiry" (we still skip the EXPIRE refresh).
    ttl_secs: u64,
    /// One TCP stream guarded by a mutex; the hand-rolled
    /// RESP client is fully synchronous.
    conn: Mutex<Option<TcpStream>>,
    /// Available flag (best-effort; reset on every
    /// successful command).
    available: std::sync::atomic::AtomicBool,
}

impl RedisBackend {
    /// Parse `redis://host:port[/db]`. Returns an error
    /// string on malformed URLs; the host surfaces it as
    /// a diagnostic and falls back to memory.
    pub fn parse_url(url: &str) -> Result<RedisEndpoint, String> {
        let rest = url
            .strip_prefix("redis://")
            .ok_or_else(|| format!("Redis URL must start with `redis://` (got {url:?})"))?;
        // Split off an optional /<db>.
        let (host_port, db) = match rest.find('/') {
            Some(i) => {
                let db_str = &rest[i + 1..];
                let db = if db_str.is_empty() {
                    0
                } else {
                    db_str
                        .parse::<i64>()
                        .map_err(|_| format!("Redis URL db is not an integer: {db_str:?}"))?
                };
                (&rest[..i], db)
            }
            None => (rest, 0),
        };
        if host_port.is_empty() {
            return Err(format!("Redis URL has no host: {url:?}"));
        }
        Ok(RedisEndpoint {
            endpoint: host_port.to_string(),
            db,
        })
    }

    /// Build a backend for `url`. `ttl_secs == 0`
    /// disables per-key expiry. The constructor does
    /// NOT dial Redis -- the first command opens the
    /// connection (and a PING in `is_available`) so a
    /// missing Redis never blocks boot.
    pub fn new(url: &str, ttl_secs: u64) -> Result<Self, String> {
        let endpoint = Self::parse_url(url)?;
        Ok(RedisBackend {
            url: url.to_string(),
            endpoint: endpoint.endpoint,
            db: endpoint.db,
            ttl_secs,
            conn: Mutex::new(None),
            available: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// Construct with the default 24h TTL.
    pub fn with_default_ttl(url: &str) -> Result<Self, String> {
        Self::new(url, REDIS_SESSION_TTL_SECS_DEFAULT)
    }

    /// Wire-level: serialize the session data map to a
    /// JSON object string Redis stores verbatim under
    /// the key. The wrap preamble's session view is
    /// already a JSON tree; we just round-trip it.
    fn serialize_view(view: &SessionView) -> String {
        let mut out = String::with_capacity(64);
        view.to_json_into(&mut out);
        out
    }

    fn parse_view_json(s: &str) -> Option<SessionView> {
        // Strip the leading `{"id":` -- easier to just
        // re-parse the whole thing and pull `id` and
        // `data` out by key. Use a tiny parser to avoid
        // pulling serde in.
        parse_session_blob(s).ok()
    }

    /// Compose the storage key for a sid.
    fn key(sid: &str) -> String {
        let mut k = String::with_capacity(REDIS_KEY_PREFIX.len() + sid.len());
        k.push_str(REDIS_KEY_PREFIX);
        k.push_str(sid);
        k
    }

    /// Build a SETEX command, optionally with EX.
    fn cmd_set(&self, sid: &str, value: &str) -> Vec<u8> {
        let key = Self::key(sid);
        if self.ttl_secs > 0 {
            // SET key value EX <ttl>
            encode_command(&[
                b"SET".to_vec(),
                key.into_bytes(),
                value.as_bytes().to_vec(),
                format!("EX").into_bytes(),
                self.ttl_secs.to_string().into_bytes(),
            ])
        } else {
            // SET key value (no expiry)
            encode_command(&[b"SET".to_vec(), key.into_bytes(), value.as_bytes().to_vec()])
        }
    }

    fn cmd_get(&self, sid: &str) -> Vec<u8> {
        let key = Self::key(sid);
        encode_command(&[b"GET".to_vec(), key.into_bytes()])
    }

    fn cmd_del(&self, sid: &str) -> Vec<u8> {
        let key = Self::key(sid);
        encode_command(&[b"DEL".to_vec(), key.into_bytes()])
    }

    fn cmd_ping(&self) -> Vec<u8> {
        // Single PING: keeps the dial handshake
        // round-trips to one. 16l only targets db 0;
        // a future slice can re-introduce SELECT on
        // connect (each command needs a matching
        // reply read, which the current reader does
        // not multi-frame).
        encode_command(&[b"PING".to_vec()])
    }

    fn dial(&self) -> Result<(), String> {
        let mut stream = TcpStream::connect(&self.endpoint)
            .map_err(|e| format!("Redis connect to {} failed: {e}", self.endpoint))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .map_err(|e| format!("Redis set_read_timeout: {e}"))?;
        stream
            .set_write_timeout(Some(Duration::from_millis(500)))
            .map_err(|e| format!("Redis set_write_timeout: {e}"))?;
        // Issue PING [+ SELECT] to confirm the link
        // works before we serve real traffic.
        write_cmd(&mut stream, &self.cmd_ping())?;
        read_simple(&mut stream)?;
        *self.conn.lock().unwrap() = Some(stream);
        self.available.store(true, Ordering::Release);
        Ok(())
    }

    /// Send one command and read its reply while holding the
    /// connection mutex for the entire request/response pair.
    /// Locking only the write and read separately would allow
    /// another caller to insert a command between them and
    /// desynchronise the RESP stream.
    fn round_trip(&self, cmd: &[u8]) -> Result<Option<Vec<u8>>, String> {
        let mut g = self.conn.lock().unwrap();
        let stream = g
            .as_mut()
            .ok_or_else(|| "Redis: no connection".to_string())?;
        write_cmd(stream, cmd)?;
        read_reply(stream)
    }

    fn round_trip_simple(&self, cmd: &[u8]) -> Result<(), String> {
        match self.round_trip(cmd)? {
            Some(_) => Ok(()),
            None => Err("Redis: nil reply to PING".to_string()),
        }
    }
}

impl SessionBackend for RedisBackend {
    fn name(&self) -> &'static str {
        "redis"
    }

    fn is_available(&self) -> bool {
        if self.conn.lock().unwrap().is_none() {
            if self.dial().is_err() {
                return false;
            }
        }
        // Already-connected: the flag tracks the last
        // successful command; a transient outage shows
        // up as `false` until the next PING.
        if !self.available.load(Ordering::Acquire) {
            // Try a PING to recover.
            let r = self.round_trip_simple(&self.cmd_ping());
            if r.is_err() {
                // Drop the connection; the next
                // command will redial.
                *self.conn.lock().unwrap() = None;
                return false;
            }
            self.available.store(true, Ordering::Release);
        }
        true
    }

    fn lookup(&self, sid: &str) -> Option<SessionView> {
        if !self.is_available() {
            return None;
        }
        let cmd = self.cmd_get(sid);
        let reply = self.round_trip(&cmd);
        match reply {
            Ok(Some(bytes)) => {
                let s = String::from_utf8_lossy(&bytes);
                match Self::parse_view_json(s.as_ref()) {
                    Some(v) => Some(v),
                    None => {
                        eprintln!("TSPv2PoC1: Redis session blob unparseable; treating as miss");
                        None
                    }
                }
            }
            Ok(None) => None,
            Err(e) => {
                eprintln!("TSPv2PoC1: Redis lookup failed: {e}");
                *self.conn.lock().unwrap() = None;
                self.available.store(false, Ordering::Release);
                None
            }
        }
    }

    fn create(&self) -> SessionView {
        let view = SessionView {
            id: mint_sid_via_counter(),
            data: BTreeMap::new(),
        };
        let value = Self::serialize_view(&view);
        let cmd = self.cmd_set(&view.id, &value);
        if !self.is_available() {
            return view;
        }
        match self.round_trip_simple(&cmd) {
            Ok(()) => view,
            Err(e) => {
                eprintln!("TSPv2PoC1: Redis create failed: {e}");
                *self.conn.lock().unwrap() = None;
                self.available.store(false, Ordering::Release);
                view
            }
        }
    }

    fn apply_writes(&self, current_sid: &str, writes: &[SessionWrite]) -> String {
        if writes.is_empty() {
            return current_sid.to_string();
        }
        if !self.is_available() {
            return current_sid.to_string();
        }
        let mut new_sid = current_sid.to_string();
        for w in writes {
            match w {
                SessionWrite::Set(name, value) => {
                    // Fetch the current view, mutate one
                    // key, write it back. This is a
                    // read-modify-write; a future slice
                    // can swap to a Redis hash (HSET)
                    // when the session grows large.
                    let current = match self.lookup(current_sid) {
                        Some(v) => v,
                        None => {
                            eprintln!("TSPv2PoC1: session Set on unknown sid dropped (key={name})");
                            continue;
                        }
                    };
                    let mut next = current.clone();
                    next.data.insert(name.clone(), value.clone());
                    let blob = Self::serialize_view(&next);
                    let cmd = self.cmd_set(&next.id, &blob);
                    if let Err(e) = self.round_trip_simple(&cmd) {
                        eprintln!("TSPv2PoC1: Redis Set write failed: {e}");
                        *self.conn.lock().unwrap() = None;
                        self.available.store(false, Ordering::Release);
                    }
                }
                SessionWrite::Delete(name) => {
                    let Some(mut current) = self.lookup(current_sid) else {
                        continue;
                    };
                    current.data.remove(name);
                    let blob = Self::serialize_view(&current);
                    let cmd = self.cmd_set(&current.id, &blob);
                    if let Err(e) = self.round_trip_simple(&cmd) {
                        eprintln!("TSPv2PoC1: Redis Delete write failed: {e}");
                        *self.conn.lock().unwrap() = None;
                        self.available.store(false, Ordering::Release);
                    }
                }
                SessionWrite::Clear => {
                    let Some(current) = self.lookup(current_sid) else {
                        continue;
                    };
                    let mut next = current.clone();
                    next.data.clear();
                    let blob = Self::serialize_view(&next);
                    let cmd = self.cmd_set(&next.id, &blob);
                    if let Err(e) = self.round_trip_simple(&cmd) {
                        eprintln!("TSPv2PoC1: Redis Clear write failed: {e}");
                        *self.conn.lock().unwrap() = None;
                        self.available.store(false, Ordering::Release);
                    }
                }
                SessionWrite::Regenerate => {
                    let Some(current) = self.lookup(current_sid) else {
                        continue;
                    };
                    let fresh = mint_sid_via_counter();
                    let next = SessionView {
                        id: fresh.clone(),
                        data: current.data.clone(),
                    };
                    let blob = Self::serialize_view(&next);
                    let cmd = self.cmd_set(&next.id, &blob);
                    if let Err(e) = self.round_trip_simple(&cmd) {
                        eprintln!("TSPv2PoC1: Redis Regenerate write failed: {e}");
                        *self.conn.lock().unwrap() = None;
                        self.available.store(false, Ordering::Release);
                        return new_sid;
                    }
                    // Old key cleanup is best-effort: a
                    // failure here just leaks one
                    // session that Redis will eventually
                    // TTL-evict.
                    let _ = self.round_trip_simple(&self.cmd_del(current_sid));
                    new_sid = fresh;
                }
                SessionWrite::Destroy => {
                    if let Err(e) = self.round_trip_simple(&self.cmd_del(current_sid)) {
                        eprintln!("TSPv2PoC1: Redis Destroy failed: {e}");
                        *self.conn.lock().unwrap() = None;
                        self.available.store(false, Ordering::Release);
                    }
                    new_sid.clear();
                }
            }
        }
        new_sid
    }
}

/// Parsed parts of a `redis://host:port[/db]` URL.
pub struct RedisEndpoint {
    pub endpoint: String,
    pub db: i64,
}

impl std::fmt::Debug for RedisBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RedisBackend {{ url: {:?}, db: {}, ttl_secs: {}, available: {} }}",
            self.url,
            self.db,
            self.ttl_secs,
            self.available.load(Ordering::Acquire)
        )
    }
}

// =====================================================================
// RESP wire format helpers
// =====================================================================

/// Encode a RESP2 command (array of bulk strings) into
/// the wire bytes the client sends.
fn encode_command(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(parts.iter().map(|p| p.len() + 16).sum::<usize>());
    out.push(b'*');
    out.extend_from_slice(parts.len().to_string().as_bytes());
    out.extend_from_slice(b"\r\n");
    for part in parts {
        out.push(b'$');
        out.extend_from_slice(part.len().to_string().as_bytes());
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(part);
        out.extend_from_slice(b"\r\n");
    }
    out
}

fn write_cmd<W: Write>(w: &mut W, cmd: &[u8]) -> Result<(), String> {
    w.write_all(cmd).map_err(|e| format!("Redis write: {e}"))?;
    w.flush().map_err(|e| format!("Redis flush: {e}"))?;
    Ok(())
}

/// Read a single RESP reply. Bulk strings return
/// `Some(Vec<u8>)`; simple strings return
/// `Some(Vec<u8>)`; nil returns `None`; errors
/// return `Err`. Arrays are drained recursively on the
/// same stream reference, so parsing never re-locks the
/// backend connection mutex.
fn read_reply<R: Read>(r: &mut R) -> Result<Option<Vec<u8>>, String> {
    let mut head = [0u8; 1];
    r.read_exact(&mut head)
        .map_err(|e| format!("Redis read head: {e}"))?;
    match head[0] {
        b'+' => {
            // Simple string: `+OK\r\n`
            let line = read_line(r)?;
            Ok(Some(line))
        }
        b'-' => {
            // Error: `-ERR ...\r\n`.
            let line = read_line(r)?;
            Err(format!("Redis ERR: {}", String::from_utf8_lossy(&line)))
        }
        b'$' => {
            // Bulk string: `$<len>\r\n<data>\r\n`
            let len_line = read_line(r)?;
            let len: i64 = std::str::from_utf8(&len_line)
                .map_err(|e| format!("Redis bulk len: {e}"))?
                .parse()
                .map_err(|e| format!("Redis bulk parse: {e}"))?;
            if len < 0 {
                return Ok(None);
            }
            let mut buf = vec![0u8; len as usize];
            r.read_exact(&mut buf)
                .map_err(|e| format!("Redis bulk body: {e}"))?;
            // Consume trailing CRLF.
            let mut tail = [0u8; 2];
            r.read_exact(&mut tail)
                .map_err(|e| format!("Redis bulk tail: {e}"))?;
            Ok(Some(buf))
        }
        b'*' => {
            // Array: `*<count>\r\n<count> replies`.
            let count: i64 = std::str::from_utf8(&read_line(r)?)
                .map_err(|e| format!("Redis multi count: {e}"))?
                .parse()
                .map_err(|e| format!("Redis multi parse: {e}"))?;
            if count < 0 {
                return Ok(None);
            }
            let mut first = None;
            for index in 0..count {
                let reply = read_reply(r)?;
                if index == 0 {
                    first = reply;
                }
            }
            Ok(first)
        }
        b':' => {
            // Integer: `:<n>\r\n`.
            let line = read_line(r)?;
            Ok(Some(line))
        }
        other => Err(format!("Redis: unknown reply tag 0x{:02x}", other)),
    }
}

fn read_simple<R: Read>(r: &mut R) -> Result<(), String> {
    match read_reply(r)? {
        Some(_) => Ok(()),
        None => Err("Redis: nil reply to PING".to_string()),
    }
}

/// Read bytes up to (and including) the next CRLF.
/// Returns the bytes before the CRLF.
fn read_line<R: Read>(r: &mut R) -> Result<Vec<u8>, String> {
    let mut buf = Vec::with_capacity(64);
    let mut prev = 0u8;
    loop {
        let mut b = [0u8; 1];
        r.read_exact(&mut b)
            .map_err(|e| format!("Redis read byte: {e}"))?;
        if prev == b'\r' && b[0] == b'\n' {
            buf.pop(); // drop the \r
            return Ok(buf);
        }
        buf.push(b[0]);
        prev = b[0];
        if buf.len() > 64 * 1024 {
            return Err("Redis: line too long".to_string());
        }
    }
}

// =====================================================================
// Session blob (de)serialisation
// =====================================================================

impl SessionView {
    /// Same wire form as `to_json` (spec 16) but writes
    /// into an existing buffer; the 16l Redis backend
    /// uses this to avoid a per-call String allocation
    /// in the hot path. Falls back to `to_json` for
    /// callers that just want an owned string.
    pub fn to_json_into(&self, out: &mut String) {
        out.push_str("{\"id\":");
        json_string_into(out, &self.id);
        out.push_str(",\"data\":{");
        let mut first = true;
        for (k, v) in &self.data {
            if !first {
                out.push(',');
            }
            first = false;
            json_string_into(out, k);
            out.push(':');
            v.to_json_into(out);
        }
        out.push_str("}}");
    }
}

impl SessionValue {
    /// In-place serialise, matching the 16k wire form.
    pub fn to_json_into(&self, out: &mut String) {
        use std::fmt::Write as _;
        match self {
            SessionValue::Null => out.push_str("null"),
            SessionValue::Bool(true) => out.push_str("true"),
            SessionValue::Bool(false) => out.push_str("false"),
            SessionValue::Number(n) => {
                let _ = write!(out, "{n}");
            }
            SessionValue::String(s) => json_string_into(out, s),
            SessionValue::Array(items) => {
                out.push('[');
                let mut first = true;
                for item in items {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    item.to_json_into(out);
                }
                out.push(']');
            }
            SessionValue::Object(entries) => {
                out.push('{');
                let mut first = true;
                for (k, v) in entries {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    json_string_into(out, k);
                    out.push(':');
                    v.to_json_into(out);
                }
                out.push('}');
            }
        }
    }
}

fn json_string_into(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Hand-rolled JSON parser for the session blob
/// (`{"id":"<sid>","data":{...}}`). Reuses no external
/// dep; sufficient for the shapes the host emits.
fn parse_session_blob(s: &str) -> Result<SessionView, String> {
    let mut p = BlobParser {
        bytes: s.as_bytes(),
        pos: 0,
    };
    p.skip_ws();
    p.expect(b'{')?;
    let mut id: Option<String> = None;
    let mut data: BTreeMap<String, SessionValue> = BTreeMap::new();
    loop {
        p.skip_ws();
        if p.peek() == Some(b'}') {
            break;
        }
        let key = p.parse_string()?;
        p.skip_ws();
        p.expect(b':')?;
        p.skip_ws();
        if key == "id" {
            id = Some(p.parse_string()?);
        } else if key == "data" {
            data = p.parse_object_map()?;
        } else {
            p.skip_value()?;
        }
        p.skip_ws();
        match p.peek() {
            Some(b',') => p.pos += 1,
            Some(b'}') => {}
            _ => return Err("blob: expected , or }".to_string()),
        }
    }
    Ok(SessionView {
        id: id.ok_or_else(|| "blob: missing id".to_string())?,
        data,
    })
}

struct BlobParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> BlobParser<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }
    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\n' || b == b'\r' || b == b'\t' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
    fn expect(&mut self, ch: u8) -> Result<(), String> {
        match self.peek() {
            Some(b) if b == ch => {
                self.pos += 1;
                Ok(())
            }
            Some(b) => Err(format!("blob: expected {} got {}", ch as char, b as char)),
            None => Err("blob: unexpected EOF".to_string()),
        }
    }
    fn parse_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let b = self.peek().ok_or_else(|| "blob: string EOF".to_string())?;
            self.pos += 1;
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = self
                        .peek()
                        .ok_or_else(|| "blob: string escape EOF".to_string())?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'/' => out.push('/'),
                        b'u' => {
                            let hex = std::str::from_utf8(
                                self.bytes
                                    .get(self.pos..self.pos + 4)
                                    .ok_or_else(|| "blob: \\u EOF".to_string())?,
                            )
                            .map_err(|e| format!("blob: \\u utf8: {e}"))?;
                            self.pos += 4;
                            let cp = u32::from_str_radix(hex, 16)
                                .map_err(|e| format!("blob: \\u parse: {e}"))?;
                            let ch = char::from_u32(cp)
                                .ok_or_else(|| "blob: \\u codepoint".to_string())?;
                            out.push(ch);
                        }
                        other => return Err(format!("blob: unknown escape \\{}", other as char)),
                    }
                }
                other => out.push(other as char),
            }
        }
    }
    fn parse_value(&mut self) -> Result<SessionValue, String> {
        match self.peek() {
            Some(b'"') => Ok(SessionValue::String(self.parse_string()?)),
            Some(b'{') => {
                let mut entries: Vec<(String, SessionValue)> = Vec::new();
                self.expect(b'{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some(b'}') {
                        self.pos += 1;
                        return Ok(SessionValue::Object(entries));
                    }
                    let k = self.parse_string()?;
                    self.skip_ws();
                    self.expect(b':')?;
                    self.skip_ws();
                    let v = self.parse_value()?;
                    entries.push((k, v));
                    self.skip_ws();
                    if self.peek() == Some(b',') {
                        self.pos += 1;
                    }
                }
            }
            Some(b'[') => {
                let mut items: Vec<SessionValue> = Vec::new();
                self.expect(b'[')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some(b']') {
                        self.pos += 1;
                        return Ok(SessionValue::Array(items));
                    }
                    items.push(self.parse_value()?);
                    self.skip_ws();
                    if self.peek() == Some(b',') {
                        self.pos += 1;
                    }
                }
            }
            Some(b't') => {
                self.pos += 4;
                Ok(SessionValue::Bool(true))
            }
            Some(b'f') => {
                self.pos += 5;
                Ok(SessionValue::Bool(false))
            }
            Some(b'n') => {
                self.pos += 4;
                Ok(SessionValue::Null)
            }
            Some(b'-') | Some(b'0'..=b'9') => {
                let start = self.pos;
                if self.peek() == Some(b'-') {
                    self.pos += 1;
                }
                while let Some(b) = self.peek() {
                    if b.is_ascii_digit()
                        || b == b'.'
                        || b == b'e'
                        || b == b'E'
                        || b == b'+'
                        || b == b'-'
                    {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                let s = std::str::from_utf8(&self.bytes[start..self.pos])
                    .map_err(|e| format!("blob: number utf8: {e}"))?;
                let n: f64 = s.parse().map_err(|e| format!("blob: number parse: {e}"))?;
                Ok(SessionValue::Number(n))
            }
            other => Err(format!("blob: unexpected value byte {:?}", other)),
        }
    }
    fn parse_object_map(&mut self) -> Result<BTreeMap<String, SessionValue>, String> {
        self.expect(b'{')?;
        let mut out: BTreeMap<String, SessionValue> = BTreeMap::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.pos += 1;
                return Ok(out);
            }
            let k = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            let v = self.parse_value()?;
            out.insert(k, v);
            self.skip_ws();
            if self.peek() == Some(b',') {
                self.pos += 1;
            }
        }
    }
    fn skip_value(&mut self) -> Result<(), String> {
        // For unknown keys we just discard the value
        // by parsing it into a `SessionValue` and
        // dropping it.
        let _ = self.parse_value()?;
        Ok(())
    }
}

/// Counter-based sid mint shared with the memory
/// backend. We use a process-global atomic so two
/// backends (or the same backend re-created during a
/// test) do not collide.
fn mint_sid_via_counter() -> String {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let high = (n >> 32) as u32;
    let low = n as u32;
    let mut buf = [0u8; 16];
    buf[0..4].copy_from_slice(&high.to_be_bytes());
    buf[4..8].copy_from_slice(&low.to_be_bytes());
    let mix = high ^ low;
    for slot in buf[8..16].chunks_mut(4) {
        let v = mix.wrapping_mul(0x9E37_79B9).wrapping_add(n as u32);
        slot.copy_from_slice(&v.to_be_bytes());
    }
    let mut out = String::with_capacity(32);
    use std::fmt::Write as _;
    for byte in buf.iter() {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::SESSION_STORE_CAP_DEFAULT;
    use std::collections::BTreeMap;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    // -------- URL parser --------

    #[test]
    fn redis_url_parses_host_port() {
        let e = RedisBackend::parse_url("redis://127.0.0.1:6379").unwrap();
        assert_eq!(e.endpoint, "127.0.0.1:6379");
        assert_eq!(e.db, 0);
    }

    #[test]
    fn redis_url_parses_db() {
        let e = RedisBackend::parse_url("redis://localhost:6379/3").unwrap();
        assert_eq!(e.endpoint, "localhost:6379");
        assert_eq!(e.db, 3);
    }

    #[test]
    fn redis_url_rejects_missing_scheme() {
        assert!(RedisBackend::parse_url("127.0.0.1:6379").is_err());
    }

    #[test]
    fn redis_url_rejects_non_integer_db() {
        assert!(RedisBackend::parse_url("redis://localhost:6379/abc").is_err());
    }

    // -------- RESP wire format --------

    #[test]
    fn resp_encodes_set_with_ex() {
        // SET key value EX 10
        let cmd = encode_command(&[
            b"SET".to_vec(),
            b"k".to_vec(),
            b"v".to_vec(),
            b"EX".to_vec(),
            b"10".to_vec(),
        ]);
        assert_eq!(
            cmd,
            b"*5\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n$2\r\nEX\r\n$2\r\n10\r\n"
        );
    }

    #[test]
    fn resp_encodes_simple_command() {
        let cmd = encode_command(&[b"PING".to_vec()]);
        assert_eq!(cmd, b"*1\r\n$4\r\nPING\r\n");
    }

    // -------- In-process RESP test server --------
    // A minimal Redis-protocol-speaking server backed by
    // a HashMap, used to validate the hand-rolled client
    // end-to-end without a real Redis. Accepts
    // SET / GET / DEL / PING / SELECT.

    fn spawn_fake_redis() -> (
        String,
        Arc<Mutex<HashMap<String, String>>>,
        thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().unwrap();
        let store: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
        let store_thread = Arc::clone(&store);
        let handle = thread::spawn(move || {
            for conn in listener.incoming() {
                let mut conn = match conn {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let _ = conn.set_read_timeout(Some(Duration::from_millis(500)));
                let _ = conn.set_write_timeout(Some(Duration::from_millis(500)));
                loop {
                    let mut head = [0u8; 1];
                    if conn.read_exact(&mut head).is_err() {
                        break;
                    }
                    if head[0] != b'*' {
                        // Not a multi-bulk -> unknown.
                        break;
                    }
                    let line = match read_line(&mut conn) {
                        Ok(l) => l,
                        Err(_) => break,
                    };
                    let n: i64 = match std::str::from_utf8(&line).ok().and_then(|s| s.parse().ok())
                    {
                        Some(n) => n,
                        None => break,
                    };
                    let mut parts: Vec<String> = Vec::new();
                    let mut parse_ok = true;
                    for _ in 0..n {
                        if conn.read_exact(&mut head).is_err() || head[0] != b'$' {
                            parse_ok = false;
                            break;
                        }
                        let len_line = match read_line(&mut conn) {
                            Ok(l) => l,
                            Err(_) => {
                                parse_ok = false;
                                break;
                            }
                        };
                        let len: i64 = match std::str::from_utf8(&len_line)
                            .ok()
                            .and_then(|s| s.parse().ok())
                        {
                            Some(l) => l,
                            None => {
                                parse_ok = false;
                                break;
                            }
                        };
                        if len < 0 {
                            parts.push(String::new());
                            continue;
                        }
                        let mut buf = vec![0u8; len as usize];
                        if conn.read_exact(&mut buf).is_err() {
                            parse_ok = false;
                            break;
                        }
                        let mut tail = [0u8; 2];
                        let _ = conn.read_exact(&mut tail);
                        parts.push(String::from_utf8_lossy(&buf).to_string());
                    }
                    if !parse_ok || parts.is_empty() {
                        break;
                    }
                    let cmd = parts[0].to_ascii_uppercase();
                    let reply = match cmd.as_str() {
                        "PING" => b"+PONG\r\n".to_vec(),
                        "SELECT" => b"+OK\r\n".to_vec(),
                        "GET" => {
                            if parts.len() < 2 {
                                b"-ERR wrong number of args\r\n".to_vec()
                            } else {
                                let key = parts[1].clone();
                                let st = store_thread.lock().unwrap();
                                match st.get(&key) {
                                    Some(v) => {
                                        let mut out = Vec::new();
                                        out.extend_from_slice(
                                            format!("${}\r\n", v.len()).as_bytes(),
                                        );
                                        out.extend_from_slice(v.as_bytes());
                                        out.extend_from_slice(b"\r\n");
                                        out
                                    }
                                    None => b"$-1\r\n".to_vec(),
                                }
                            }
                        }
                        "SET" => {
                            if parts.len() < 3 {
                                b"-ERR wrong number of args\r\n".to_vec()
                            } else {
                                let key = parts[1].clone();
                                let val = parts[2].clone();
                                let mut st = store_thread.lock().unwrap();
                                st.insert(key, val);
                                b"+OK\r\n".to_vec()
                            }
                        }
                        "DEL" => {
                            let mut st = store_thread.lock().unwrap();
                            let mut removed = 0i64;
                            for p in &parts[1..] {
                                if st.remove(p).is_some() {
                                    removed += 1;
                                }
                            }
                            format!(":{}\r\n", removed).into_bytes()
                        }
                        other => format!("-ERR unknown command {other}\r\n").into_bytes(),
                    };
                    if conn.write_all(&reply).is_err() {
                        break;
                    }
                    let _ = conn.flush();
                }
            }
        });
        (format!("redis://{}", addr), store, handle)
    }

    // -------- Session blob (de)serialisation --------

    #[test]
    fn session_blob_round_trips() {
        let mut data: BTreeMap<String, SessionValue> = BTreeMap::new();
        data.insert(
            "name".to_string(),
            SessionValue::String("alice".to_string()),
        );
        data.insert("n".to_string(), SessionValue::Number(7.0));
        data.insert("flag".to_string(), SessionValue::Bool(true));
        let v = SessionView {
            id: "deadbeef".to_string(),
            data,
        };
        let mut s = String::new();
        v.to_json_into(&mut s);
        let back = parse_session_blob(&s).unwrap();
        assert_eq!(back.id, "deadbeef");
        assert_eq!(
            back.data.get("name").unwrap(),
            &SessionValue::String("alice".to_string())
        );
        assert_eq!(back.data.get("n").unwrap(), &SessionValue::Number(7.0));
        assert_eq!(back.data.get("flag").unwrap(), &SessionValue::Bool(true));
    }

    #[test]
    fn session_blob_round_trips_nested() {
        let mut inner: BTreeMap<String, SessionValue> = BTreeMap::new();
        inner.insert("a".to_string(), SessionValue::Number(1.0));
        let mut data: BTreeMap<String, SessionValue> = BTreeMap::new();
        data.insert(
            "obj".to_string(),
            SessionValue::Object(inner.into_iter().map(|(k, v)| (k, v)).collect::<Vec<_>>()),
        );
        data.insert(
            "arr".to_string(),
            SessionValue::Array(vec![SessionValue::Bool(false), SessionValue::Null]),
        );
        let v = SessionView {
            id: "x".to_string(),
            data,
        };
        let mut s = String::new();
        v.to_json_into(&mut s);
        let back = parse_session_blob(&s).unwrap();
        assert_eq!(back.id, "x");
        match back.data.get("arr").unwrap() {
            SessionValue::Array(items) => {
                assert_eq!(items.len(), 2);
            }
            other => panic!("expected Array, got {other:?}"),
        }
    }

    // -------- Memory backend contract --------

    #[test]
    fn memory_backend_basic_round_trip() {
        let b = MemoryBackend::new(8);
        let v = b.create();
        let id = v.id.clone();
        let next = b.apply_writes(
            &id,
            &[
                SessionWrite::Set("k".to_string(), SessionValue::String("v".to_string())),
                SessionWrite::Set("n".to_string(), SessionValue::Number(2.0)),
            ],
        );
        assert_eq!(next, id);
        let view = b.lookup(&id).unwrap();
        assert_eq!(
            view.data.get("k").unwrap(),
            &SessionValue::String("v".to_string())
        );
    }

    #[test]
    fn memory_backend_destroy_removes_row() {
        let b = MemoryBackend::new(4);
        let v = b.create();
        let id = v.id;
        let next = b.apply_writes(&id, &[SessionWrite::Destroy]);
        assert!(next.is_empty());
        assert!(b.lookup(&id).is_none());
    }

    #[test]
    fn memory_backend_regenerate_keeps_data() {
        let b = MemoryBackend::new(4);
        let v = b.create();
        let old = v.id;
        b.apply_writes(
            &old,
            &[SessionWrite::Set(
                "k".to_string(),
                SessionValue::String("v".to_string()),
            )],
        );
        let new_id = b.apply_writes(&old, &[SessionWrite::Regenerate]);
        assert_ne!(old, new_id);
        let v = b.lookup(&new_id).unwrap();
        assert_eq!(
            v.data.get("k").unwrap(),
            &SessionValue::String("v".to_string())
        );
    }

    // -------- Redis backend against the in-process fake --------

    #[test]
    fn redis_backend_round_trips_via_fake_server() {
        let (url, store, _h) = spawn_fake_redis();
        let b = RedisBackend::with_default_ttl(&url).expect("parse url");
        let v = b.create();
        let id = v.id.clone();
        // apply writes
        let next = b.apply_writes(
            &id,
            &[
                SessionWrite::Set("k".to_string(), SessionValue::String("v".to_string())),
                SessionWrite::Set("n".to_string(), SessionValue::Number(7.0)),
            ],
        );
        assert_eq!(next, id);
        // The fake store has the entry.
        let st = store.lock().unwrap();
        let key = format!("tsp:session:{id}");
        assert!(st.contains_key(&key), "fake store missing {key}");
        drop(st);
        // Read back via the backend.
        let view = b.lookup(&id).expect("lookup after write");
        assert_eq!(
            view.data.get("k").unwrap(),
            &SessionValue::String("v".to_string())
        );
        assert_eq!(view.data.get("n").unwrap(), &SessionValue::Number(7.0));
    }

    #[test]
    fn redis_backend_destroy_deletes_key() {
        let (url, store, _h) = spawn_fake_redis();
        let b = RedisBackend::with_default_ttl(&url).expect("parse url");
        let v = b.create();
        let id = v.id.clone();
        let next = b.apply_writes(&id, &[SessionWrite::Destroy]);
        assert!(next.is_empty());
        let st = store.lock().unwrap();
        assert!(!st.contains_key(&format!("tsp:session:{id}")));
    }

    #[test]
    fn redis_backend_regenerate_moves_data() {
        let (url, store, _h) = spawn_fake_redis();
        let b = RedisBackend::with_default_ttl(&url).expect("parse url");
        let v = b.create();
        let old = v.id;
        b.apply_writes(
            &old,
            &[SessionWrite::Set(
                "k".to_string(),
                SessionValue::String("v".to_string()),
            )],
        );
        let new_id = b.apply_writes(&old, &[SessionWrite::Regenerate]);
        assert_ne!(old, new_id);
        let st = store.lock().unwrap();
        assert!(!st.contains_key(&format!("tsp:session:{old}")));
        let new_key = format!("tsp:session:{new_id}");
        assert!(st.contains_key(&new_key));
    }

    #[test]
    fn redis_backend_lookup_miss_returns_none() {
        let (url, _store, _h) = spawn_fake_redis();
        let b = RedisBackend::with_default_ttl(&url).expect("parse url");
        assert!(b.lookup("nonexistent-sid").is_none());
    }

    #[test]
    fn redis_backend_unavailable_when_endpoint_dead() {
        // Port 1 is reserved and not connectable on
        // every platform; use a high port that is
        // almost certainly closed. The test only
        // checks that `is_available` flips to false.
        let b = RedisBackend::with_default_ttl("redis://127.0.0.1:1")
            .expect("parse url even though endpoint is dead");
        assert!(!b.is_available());
    }

    // Smoke check: the global counter used for sid
    // minting advances across calls.
    #[test]
    fn mint_sid_is_unique() {
        let a = mint_sid_via_counter();
        let b = mint_sid_via_counter();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        assert_eq!(b.len(), 32);
    }

    // Reference the constant to silence the warning
    // when the test mod is the only user.
    #[test]
    fn cap_default_constant_is_sane() {
        assert!(SESSION_STORE_CAP_DEFAULT > 0);
    }
}
