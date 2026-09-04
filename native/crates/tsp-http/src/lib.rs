//! TSP-owned HTTP/1.1 wire boundary.
//!
//! The parser intentionally handles one complete request at a time. The
//! connection owner decides whether to keep the socket alive; this crate does
//! not depend on uWebSockets, libuv, or Bun HTTP types.

use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

pub use tsp_core::{BodyEnvelope, PROTOCOL_VERSION, Request, Response};

const DEFAULT_MAX_HEADER_BYTES: usize = 64 * 1024;
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Incomplete,
    HeaderTooLarge,
    MalformedRequestLine,
    InvalidMethod,
    InvalidTarget,
    InvalidHeader,
    InvalidContentLength,
    BodyTooLarge { limit: usize },
    UnsupportedTransferEncoding,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete => formatter.write_str("request is incomplete"),
            Self::HeaderTooLarge => {
                formatter.write_str("request headers exceed the configured limit")
            }
            Self::MalformedRequestLine => formatter.write_str("malformed HTTP request line"),
            Self::InvalidMethod => formatter.write_str("invalid HTTP method"),
            Self::InvalidTarget => formatter.write_str("invalid request target"),
            Self::InvalidHeader => formatter.write_str("invalid HTTP header"),
            Self::InvalidContentLength => formatter.write_str("invalid Content-Length"),
            Self::BodyTooLarge { limit } => write!(formatter, "request body exceeds {limit} bytes"),
            Self::UnsupportedTransferEncoding => {
                formatter.write_str("chunked transfer encoding is not supported yet")
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_request(bytes: &[u8], max_body_bytes: usize) -> Result<(Request, usize), ParseError> {
    parse_request_with_header_limit(bytes, max_body_bytes, DEFAULT_MAX_HEADER_BYTES)
}

pub fn parse_request_with_header_limit(
    bytes: &[u8],
    max_body_bytes: usize,
    max_header_bytes: usize,
) -> Result<(Request, usize), ParseError> {
    let Some(header_end) = find_header_end(bytes) else {
        if bytes.len() > max_header_bytes {
            return Err(ParseError::HeaderTooLarge);
        }
        return Err(ParseError::Incomplete);
    };
    if header_end > max_header_bytes {
        return Err(ParseError::HeaderTooLarge);
    }

    let header_text =
        std::str::from_utf8(&bytes[..header_end]).map_err(|_| ParseError::InvalidHeader)?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or(ParseError::MalformedRequestLine)?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or(ParseError::MalformedRequestLine)?;
    let target = request_parts
        .next()
        .ok_or(ParseError::MalformedRequestLine)?;
    let version = request_parts
        .next()
        .ok_or(ParseError::MalformedRequestLine)?;
    if request_parts.next().is_some() {
        return Err(ParseError::MalformedRequestLine);
    }
    if !is_token(method) {
        return Err(ParseError::InvalidMethod);
    }
    if !(target.starts_with('/') || target == "*") {
        return Err(ParseError::InvalidTarget);
    }
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(ParseError::MalformedRequestLine);
    }

    let mut headers = Vec::new();
    let mut content_length = None;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(ParseError::InvalidHeader);
        };
        if !is_token(name) {
            return Err(ParseError::InvalidHeader);
        }
        let name = name.to_ascii_lowercase();
        let value = value.trim();
        if value.bytes().any(|byte| byte == b'\r' || byte == b'\n') {
            return Err(ParseError::InvalidHeader);
        }
        if name == "content-length" {
            let parsed = value
                .parse::<usize>()
                .map_err(|_| ParseError::InvalidContentLength)?;
            if let Some(previous) = content_length {
                if previous != parsed {
                    return Err(ParseError::InvalidContentLength);
                }
            }
            content_length = Some(parsed);
        }
        if name == "transfer-encoding" && !value.eq_ignore_ascii_case("identity") {
            return Err(ParseError::UnsupportedTransferEncoding);
        }
        headers.push((name, value.to_owned()));
    }

    let body_length = content_length.unwrap_or(0);
    if body_length > max_body_bytes {
        return Err(ParseError::BodyTooLarge {
            limit: max_body_bytes,
        });
    }
    let body_start = header_end + 4;
    let request_end = body_start
        .checked_add(body_length)
        .ok_or(ParseError::BodyTooLarge {
            limit: max_body_bytes,
        })?;
    if bytes.len() < request_end {
        return Err(ParseError::Incomplete);
    }

    Ok((
        Request {
            version: PROTOCOL_VERSION,
            request_id: format!("http-{}", REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)),
            generation: None,
            method: method.to_owned(),
            target: target.to_owned(),
            http_version: version.to_owned(),
            headers,
            body: BodyEnvelope::from(bytes[body_start..request_end].to_vec()),
        },
        request_end,
    ))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn is_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

pub trait ResponseSerializeExt {
    fn serialize(&self) -> Result<Vec<u8>, ResponseError>;
}

impl ResponseSerializeExt for Response {
    fn serialize(&self) -> Result<Vec<u8>, ResponseError> {
        if !(100..=999).contains(&self.status) {
            return Err(ResponseError::InvalidStatus);
        }
        let body = self.body.as_bytes();
        let mut output = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status,
            reason_phrase(self.status)
        )
        .into_bytes();
        let mut has_content_length = false;
        let mut has_connection = false;
        for (name, value) in &self.headers {
            if !is_token(name) || value.bytes().any(|byte| byte == b'\r' || byte == b'\n') {
                return Err(ResponseError::InvalidHeader);
            }
            has_content_length |= name.eq_ignore_ascii_case("content-length");
            has_connection |= name.eq_ignore_ascii_case("connection");
            output.extend_from_slice(name.as_bytes());
            output.extend_from_slice(b": ");
            output.extend_from_slice(value.as_bytes());
            output.extend_from_slice(b"\r\n");
        }
        if !has_content_length {
            output.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
        }
        if !has_connection {
            output.extend_from_slice(b"Connection: close\r\n");
        }
        output.extend_from_slice(b"\r\n");
        output.extend_from_slice(body);
        Ok(output)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseError {
    InvalidStatus,
    InvalidHeader,
}

#[derive(Debug, Clone, Copy)]
pub struct ServerLimits {
    pub max_header_bytes: usize,
    pub max_body_bytes: usize,
}

impl Default for ServerLimits {
    fn default() -> Self {
        Self {
            max_header_bytes: DEFAULT_MAX_HEADER_BYTES,
            max_body_bytes: 16 * 1024 * 1024,
        }
    }
}

/// Small HTTP server used by the native runtime. Each connection is isolated
/// on one OS thread for now; the worker pool remains a separate concern. This
/// keeps socket ownership and JSC ownership from sharing a thread implicitly.
pub struct Server {
    listener: TcpListener,
    limits: ServerLimits,
}

impl Server {
    pub fn bind(address: impl std::net::ToSocketAddrs, limits: ServerLimits) -> io::Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(address)?,
            limits,
        })
    }

    pub fn local_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    pub fn run<H>(self, handler: H) -> io::Result<()>
    where
        H: Fn(Request) -> Response + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        for stream in self.listener.incoming() {
            let mut stream = stream?;
            let handler = Arc::clone(&handler);
            let limits = self.limits;
            thread::spawn(move || {
                let result = serve_connection(&mut stream, limits, |request| handler(request));
                if let Err(error) = result {
                    let _ = write_internal_error(&mut stream, &error);
                }
            });
        }
        Ok(())
    }
}

fn serve_connection(
    stream: &mut TcpStream,
    limits: ServerLimits,
    handler: impl FnOnce(Request) -> Response,
) -> io::Result<()> {
    let mut bytes = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 8192];
    let (request, _) = loop {
        match parse_request_with_header_limit(
            &bytes,
            limits.max_body_bytes,
            limits.max_header_bytes,
        ) {
            Ok(request) => break request,
            Err(ParseError::Incomplete) => {
                let count = stream.read(&mut chunk)?;
                if count == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        ParseError::Incomplete,
                    ));
                }
                bytes.extend_from_slice(&chunk[..count]);
            }
            Err(error) => return Err(io::Error::new(io::ErrorKind::InvalidData, error)),
        }
        if bytes.len()
            > limits
                .max_header_bytes
                .saturating_add(limits.max_body_bytes)
                .saturating_add(4)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                ParseError::BodyTooLarge {
                    limit: limits.max_body_bytes,
                },
            ));
        }
    };
    stream.write_all(&handler(request).serialize().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("response serialization failed: {error:?}"),
        )
    })?)?;
    stream.flush()
}

fn write_internal_error(stream: &mut TcpStream, error: &io::Error) -> io::Result<()> {
    let status = match error.kind() {
        io::ErrorKind::InvalidData => 400,
        io::ErrorKind::UnexpectedEof => 400,
        _ => 500,
    };
    stream.write_all(
        &Response::new(status, error.to_string())
            .serialize()
            .unwrap_or_default(),
    )
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "TSP Response",
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread;

    use super::*;

    #[test]
    fn parses_headers_and_a_binary_body() {
        let input = b"POST /upload HTTP/1.1\r\nHost: example.test\r\nContent-Length: 3\r\nX-Test: first\r\nX-Test: second\r\n\r\n\x00\x01\x02tail";
        let (request, consumed) = parse_request(input, 16).unwrap();
        assert_eq!(consumed, input.len() - 4);
        assert_eq!(request.method, "POST");
        assert_eq!(request.target, "/upload");
        assert_eq!(request.header("host"), Some("example.test"));
        assert_eq!(request.body.as_bytes(), [0, 1, 2]);
        assert_eq!(request.version, PROTOCOL_VERSION);
        assert!(request.request_id.starts_with("http-"));
    }

    #[test]
    fn incomplete_and_oversized_bodies_are_distinct() {
        let incomplete = b"POST / HTTP/1.1\r\nContent-Length: 4\r\n\r\nabc";
        assert_eq!(parse_request(incomplete, 8), Err(ParseError::Incomplete));
        let oversized = b"POST / HTTP/1.1\r\nContent-Length: 4\r\n\r\nabcd";
        assert_eq!(
            parse_request(oversized, 3),
            Err(ParseError::BodyTooLarge { limit: 3 })
        );
    }

    #[test]
    fn rejects_ambiguous_lengths_and_chunked_requests() {
        let duplicate = b"POST / HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\na";
        assert_eq!(
            parse_request(duplicate, 8),
            Err(ParseError::InvalidContentLength)
        );
        let chunked = b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n1\r\na\r\n0\r\n\r\n";
        assert_eq!(
            parse_request(chunked, 8),
            Err(ParseError::UnsupportedTransferEncoding)
        );
    }

    #[test]
    fn serializes_a_response_with_safe_defaults() {
        let mut response = Response::new(200, b"ok".to_vec());
        response
            .headers
            .push(("Content-Type".into(), "text/plain".into()));
        let output = String::from_utf8(response.serialize().unwrap()).unwrap();
        assert!(output.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(output.contains("Content-Length: 2\r\n"));
        assert!(output.ends_with("\r\nok"));
    }

    #[test]
    fn serves_one_complete_application_request_over_tcp() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            serve_connection(&mut stream, ServerLimits::default(), |request| {
                Response::new(200, format!("{} {}", request.method, request.target))
            })
            .unwrap();
        });

        let mut client = TcpStream::connect(address).unwrap();
        client
            .write_all(b"GET /native?q=1 HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).unwrap();
        server.join().unwrap();
        let response = String::from_utf8(response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with("\r\nGET /native?q=1"));
    }
}
