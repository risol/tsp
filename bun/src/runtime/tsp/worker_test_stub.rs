//! Protocol-only worker used by Worker Manager integration tests.
//!
//! This binary exercises the master process boundary without initializing
//! Bun. The real embedded worker is covered by the v2.4 smoke test.

#[path = "worker/protocol.rs"]
#[allow(unreachable_pub, dead_code)]
mod protocol;

use protocol::{ExecuteResponse, Message, ProtocolError};
use std::io::{Read, Write};

fn main() -> std::process::ExitCode {
    if std::env::args().any(|argument| argument == "--tsp-worker") {
        return run();
    }
    std::process::ExitCode::from(2)
}

fn run() -> std::process::ExitCode {
    #[cfg(unix)]
    {
        let Some(path) = std::env::var_os("TSP_WORKER_SOCKET") else {
            return std::process::ExitCode::from(2);
        };
        let Ok(stream) = std::os::unix::net::UnixStream::connect(path) else {
            return std::process::ExitCode::from(2);
        };
        return serve(stream);
    }
    #[cfg(not(unix))]
    {
        let Ok(endpoint) = std::env::var("TSP_WORKER_SOCKET") else {
            return std::process::ExitCode::from(2);
        };
        let Ok(stream) = std::net::TcpStream::connect(endpoint) else {
            return std::process::ExitCode::from(2);
        };
        return serve(stream);
    }
}

fn serve<S>(mut stream: S) -> std::process::ExitCode
where
    S: Read + Write,
{
    if !matches!(Message::read_from(&mut stream), Ok(Message::Hello)) {
        return std::process::ExitCode::from(2);
    }
    // Publish process info before sending Ready so the master can
    // read the file once it observes the Ready message. The
    // `--tsp-worker-info=<path>` argument is opt-in; the test
    // harness passes it only for process-model assertions, and the
    // protocol-only smoke test does not. We accept either the CLI
    // flag or the legacy `TSP_WORKER_INFO_PATH` env var so the
    // master API can pick whichever is more convenient; the CLI
    // flag wins when both are set so parallel tests can race
    // without leaking the env var between them.
    if let Some(path) = worker_info_path_from_args() {
        write_process_info(path.as_os_str());
    } else if let Some(info_path) = std::env::var_os("TSP_WORKER_INFO_PATH") {
        write_process_info(&info_path);
    }
    if (Message::Ready {
        worker_id: std::process::id() as u64,
    })
    .write_to(&mut stream)
    .is_err()
    {
        return std::process::ExitCode::from(2);
    }

    loop {
        let message = match Message::read_from(&mut stream) {
            Ok(message) => message,
            Err(ProtocolError::Truncated) => return std::process::ExitCode::SUCCESS,
            Err(_) => return std::process::ExitCode::from(2),
        };
        match message {
            Message::Execute { id, request } => {
                let script = String::from_utf8_lossy(&request.script);
                if request.path.contains("crash") || script.contains("__TSP_TEST_CRASH__") {
                    std::process::exit(17);
                }
                if request.path.contains("sleep") || script.contains("__TSP_TEST_SLEEP__") {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                }
                let body = if request.script.is_empty() {
                    request.path.into_bytes()
                } else {
                    request.script
                };
                if (Message::Response {
                    id,
                    response: ExecuteResponse {
                        status: 200,
                        headers: Vec::new(),
                        body,
                    },
                })
                .write_to(&mut stream)
                .is_err()
                {
                    return std::process::ExitCode::from(2);
                }
            }
            Message::Heartbeat { id } => {
                if (Message::Heartbeat { id }).write_to(&mut stream).is_err() {
                    return std::process::ExitCode::from(2);
                }
            }
            Message::Cancel { .. } => {}
            Message::Shutdown => return std::process::ExitCode::SUCCESS,
            _ => return std::process::ExitCode::from(2),
        }
    }
}

// ---------------------------------------------------------------------------
// Process-info publication for process-model tests.
//
// The stub writes a small JSON document to the path given in
// `TSP_WORKER_INFO_PATH` before sending the Ready message. The
// integration test harness reads the file via [`read_process_info`]
// and asserts against the fields. The parser is intentionally
// dependency-free (no serde) and lives next to the writer so the
// writer / parser can never drift apart.
// ---------------------------------------------------------------------------

/// Snapshot of the fields a process-model test can assert against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub exe_path: std::path::PathBuf,
    pub argv: Vec<String>,
}

/// Read the JSON file the stub wrote. Used by the integration test
/// harness.
pub fn read_process_info(path: &std::path::Path) -> Option<ParsedProcessInfo> {
    let raw = std::fs::read_to_string(path).ok()?;
    let (pid, ppid, exe, argv) = parse_process_info(&raw)?;
    Some(ParsedProcessInfo {
        pid,
        ppid,
        exe_path: std::path::PathBuf::from(exe),
        argv,
    })
}

/// Publish the worker's process information. Format: hand-rolled
/// JSON, see `parser_tests::roundtrip_preserves_fields` for the
/// round-trip shape.
fn write_process_info(path: &std::ffi::OsStr) {
    use std::io::Write;
    let pid = std::process::id();
    let ppid: u32 = parent_pid();
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_owned()))
        .unwrap_or_default();
    let argv: Vec<String> = std::env::args().collect();
    let mut json = String::new();
    json.push('{');
    json.push_str(&format!("\"pid\":{pid},"));
    json.push_str(&format!("\"ppid\":{ppid},"));
    json.push_str("\"exe\":");
    push_json_string(&mut json, &exe);
    json.push(',');
    json.push_str("\"argv\":[");
    for (index, arg) in argv.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        push_json_string(&mut json, arg);
    }
    json.push_str("]}");
    if let Ok(mut file) = std::fs::File::create(path) {
        let _ = file.write_all(json.as_bytes());
        let _ = file.flush();
    }
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('"');
}

#[cfg(unix)]
fn parent_pid() -> u32 {
    // Safety: getppid has no preconditions and is async-signal-safe.
    let pid: i32 = unsafe { libc::getppid() };
    pid as u32
}

/// Pull the info-file path out of the command line. The flag is
/// `--tsp-worker-info=<path>` so the stub does not have to
/// distinguish "arg with value" from "env var" — passing it as a
/// per-spawn CLI argument is what makes the parallel test runner
/// race-safe.
fn worker_info_path_from_args() -> Option<std::path::PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(rest) = arg.strip_prefix("--tsp-worker-info=") {
            return Some(std::path::PathBuf::from(rest));
        }
        if arg == "--tsp-worker-info" {
            return args.next().map(std::path::PathBuf::from);
        }
    }
    None
}

#[cfg(not(unix))]
fn parent_pid() -> u32 {
    // The Windows process tree is observable through
    // NtQueryInformationProcess / Toolhelp snapshot APIs. The
    // process-model tests that need the Windows parent PID pass
    // TSP_MASTER_PID through the env and assert against it
    // directly; the JSON file's ppid is documented as best-effort
    // for cross-platform tests.
    0
}

fn parse_process_info(raw: &str) -> Option<(u32, u32, String, Vec<String>)> {
    let pid = extract_u32_field(raw, "pid")?;
    let ppid = extract_u32_field(raw, "ppid").unwrap_or(0);
    let exe = extract_string_field(raw, "exe")?;
    let argv = extract_string_array(raw, "argv");
    Some((pid, ppid, exe, argv))
}

fn skip_whitespace(json: &str, mut index: usize) -> usize {
    let bytes = json.as_bytes();
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b' ' || byte == b'\t' || byte == b'\n' || byte == b'\r' {
            index += 1;
        } else {
            break;
        }
    }
    index
}

fn find_field(json: &str, field: &str) -> Option<usize> {
    let needle = format!("\"{field}\":");
    let start = json.find(&needle)? + needle.len();
    Some(skip_whitespace(json, start))
}

fn extract_u32_field(json: &str, field: &str) -> Option<u32> {
    let index = find_field(json, field)?;
    let bytes = json.as_bytes();
    let mut end = index;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == index {
        return None;
    }
    json[index..end].parse().ok()
}

fn extract_string_field(json: &str, field: &str) -> Option<String> {
    let index = find_field(json, field)?;
    let (value, _next) = decode_json_string_at(json, index)?;
    Some(value)
}

fn extract_string_array(json: &str, field: &str) -> Vec<String> {
    let index = match find_field(json, field) {
        Some(index) => index,
        None => return Vec::new(),
    };
    let bytes = json.as_bytes();
    if bytes.get(index) != Some(&b'[') {
        return Vec::new();
    }
    let mut cursor = index + 1;
    let mut out = Vec::new();
    while cursor < bytes.len() {
        cursor = skip_whitespace(json, cursor);
        if bytes.get(cursor) == Some(&b']') {
            return out;
        }
        if bytes.get(cursor) != Some(&b'"') {
            break;
        }
        match decode_json_string_at(json, cursor) {
            Some((value, next)) => {
                out.push(value);
                cursor = skip_whitespace(json, next);
                if bytes.get(cursor) == Some(&b',') {
                    cursor = skip_whitespace(json, cursor + 1);
                }
            }
            None => break,
        }
    }
    out
}

fn decode_json_string_at(json: &str, index: usize) -> Option<(String, usize)> {
    let bytes = json.as_bytes();
    if bytes.get(index) != Some(&b'"') {
        return None;
    }
    let mut cursor = index + 1;
    let mut out = String::new();
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte == b'\\' {
            cursor += 1;
            let resolved = match *bytes.get(cursor)? {
                b'"' => '"',
                b'\\' => '\\',
                b'n' => '\n',
                b't' => '\t',
                b'r' => '\r',
                other => other as char,
            };
            out.push(resolved);
            cursor += 1;
            continue;
        }
        if byte == b'"' {
            return Some((out, cursor + 1));
        }
        let remainder = &json[cursor..];
        let mut chars = remainder.chars();
        let character = chars.next()?;
        out.push(character);
        cursor += character.len_utf8();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_sample() -> String {
        let mut json = String::new();
        json.push_str("{\"pid\":4242,");
        json.push_str("\"ppid\":99,");
        json.push_str("\"exe\":");
        push_json_string(&mut json, "C:\\Path With Space\\bin.exe");
        json.push(',');
        json.push_str("\"argv\":[");
        push_json_string(&mut json, "C:\\bin\\app.exe");
        json.push(',');
        push_json_string(&mut json, "--tsp-worker");
        json.push_str("]}");
        json
    }

    #[test]
    fn roundtrip_preserves_fields() {
        let json = write_sample();
        let (pid, ppid, exe, argv) =
            parse_process_info(&json).expect("parser should round-trip");
        assert_eq!(pid, 4242);
        assert_eq!(ppid, 99);
        assert_eq!(exe, "C:\\Path With Space\\bin.exe");
        assert_eq!(argv, vec!["C:\\bin\\app.exe", "--tsp-worker"]);
    }

    #[test]
    fn missing_optional_field_defaults_ppid_to_zero() {
        let json = "{\"pid\":1,\"exe\":\"x\",\"argv\":[]}";
        let (pid, ppid, exe, argv) = parse_process_info(json).unwrap();
        assert_eq!(pid, 1);
        assert_eq!(ppid, 0);
        assert_eq!(exe, "x");
        assert!(argv.is_empty());
    }

    #[test]
    fn write_and_read_file_round_trip() {
        let temp = std::env::temp_dir().join(format!(
            "tsp-stub-info-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_process_info(temp.as_os_str());
        let parsed = read_process_info(&temp).expect("file should parse");
        let _ = std::fs::remove_file(&temp);
        // pid / ppid / argv are process-specific, but the file
        // shape and the exe path should always be sane.
        assert!(parsed.pid > 0);
        assert!(!parsed.exe_path.as_os_str().is_empty());
        assert!(!parsed.argv.is_empty(), "argv should include the program name");
    }
}
