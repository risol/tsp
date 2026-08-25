//! Versioned binary protocol between the TSP master and a worker process.
//!
//! The protocol has no Bun or JSC types in it. This is important because the
//! master must not link or initialize Bun merely to schedule work. The worker
//! owns all runtime-specific state behind this process boundary.

use std::fmt;
use std::io::{self, Read, Write};

pub const MAGIC: [u8; 4] = *b"TSPW";
pub const VERSION: u16 = 2;
pub const HEADER_LEN: usize = 20;
pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Hello = 1,
    Ready = 2,
    Execute = 3,
    Response = 4,
    Cancel = 5,
    Shutdown = 6,
    Heartbeat = 7,
    Error = 8,
}

impl MessageType {
    fn from_byte(byte: u8) -> Result<Self, ProtocolError> {
        match byte {
            1 => Ok(Self::Hello),
            2 => Ok(Self::Ready),
            3 => Ok(Self::Execute),
            4 => Ok(Self::Response),
            5 => Ok(Self::Cancel),
            6 => Ok(Self::Shutdown),
            7 => Ok(Self::Heartbeat),
            8 => Ok(Self::Error),
            _ => Err(ProtocolError::UnknownMessageType(byte)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteRequest {
    pub application: String,
    pub method: String,
    pub path: String,
    /// Absolute request deadline in milliseconds since the Unix epoch. Zero
    /// means that the caller did not provide a deadline.
    pub deadline_ms: u64,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Generated executable module bytes. The master prepares the module;
    /// the worker owns its materialization and execution.
    pub script: Vec<u8>,
    /// Serialized request context retained as an explicit IPC field so the
    /// worker contract carries the full request independently of the wrapper.
    pub context_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Hello,
    Ready {
        worker_id: u64,
    },
    Execute {
        id: u64,
        request: ExecuteRequest,
    },
    Response {
        id: u64,
        response: ExecuteResponse,
    },
    Cancel {
        id: u64,
    },
    Shutdown,
    Heartbeat {
        id: u64,
    },
    Error {
        id: u64,
        code: String,
        message: String,
    },
}

#[derive(Debug)]
pub enum ProtocolError {
    Io(io::Error),
    InvalidMagic([u8; 4]),
    UnsupportedVersion(u16),
    UnknownMessageType(u8),
    FrameTooLarge(usize),
    Truncated,
    InvalidField(&'static str),
    InvalidUtf8,
    RemoteError(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "worker protocol I/O failed: {error}"),
            Self::InvalidMagic(magic) => write!(f, "invalid worker protocol magic: {magic:?}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported worker protocol version: {version}")
            }
            Self::UnknownMessageType(kind) => write!(f, "unknown worker message type: {kind}"),
            Self::FrameTooLarge(size) => write!(f, "worker frame is too large: {size} bytes"),
            Self::Truncated => write!(f, "truncated worker protocol frame"),
            Self::InvalidField(field) => write!(f, "invalid worker protocol field: {field}"),
            Self::InvalidUtf8 => write!(f, "worker protocol string is not UTF-8"),
            Self::RemoteError(message) => write!(f, "worker returned an error: {message}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<io::Error> for ProtocolError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl Message {
    fn kind(&self) -> MessageType {
        match self {
            Self::Hello => MessageType::Hello,
            Self::Ready { .. } => MessageType::Ready,
            Self::Execute { .. } => MessageType::Execute,
            Self::Response { .. } => MessageType::Response,
            Self::Cancel { .. } => MessageType::Cancel,
            Self::Shutdown => MessageType::Shutdown,
            Self::Heartbeat { .. } => MessageType::Heartbeat,
            Self::Error { .. } => MessageType::Error,
        }
    }

    fn id(&self) -> u64 {
        match self {
            Self::Execute { id, .. }
            | Self::Response { id, .. }
            | Self::Cancel { id }
            | Self::Heartbeat { id }
            | Self::Error { id, .. } => *id,
            Self::Hello | Self::Ready { .. } | Self::Shutdown => 0,
        }
    }

    /// Serialize one complete frame. The header is:
    /// `magic[4], version[u16], type[u8], reserved[u8], payload_len[u32], id[u64]`.
    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut payload = Vec::new();
        match self {
            Self::Hello | Self::Shutdown => {}
            Self::Ready { worker_id } => put_u64(&mut payload, *worker_id),
            Self::Execute { request, .. } => encode_request(&mut payload, request)?,
            Self::Response { response, .. } => encode_response(&mut payload, response)?,
            Self::Cancel { .. } | Self::Heartbeat { .. } => {}
            Self::Error { code, message, .. } => {
                put_string(&mut payload, code)?;
                put_string(&mut payload, message)?;
            }
        }
        let frame_size = HEADER_LEN
            .checked_add(payload.len())
            .ok_or(ProtocolError::FrameTooLarge(usize::MAX))?;
        if frame_size > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge(frame_size));
        }

        let mut frame = Vec::with_capacity(frame_size);
        frame.extend_from_slice(&MAGIC);
        frame.extend_from_slice(&VERSION.to_be_bytes());
        frame.push(self.kind() as u8);
        frame.push(0);
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(&self.id().to_be_bytes());
        frame.extend_from_slice(&payload);
        Ok(frame)
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), ProtocolError> {
        writer.write_all(&self.encode()?)?;
        writer.flush()?;
        Ok(())
    }

    pub fn decode_frame(frame: &[u8]) -> Result<Self, ProtocolError> {
        if frame.len() < HEADER_LEN {
            return Err(ProtocolError::Truncated);
        }
        let mut magic = [0; 4];
        magic.copy_from_slice(&frame[..4]);
        if magic != MAGIC {
            return Err(ProtocolError::InvalidMagic(magic));
        }
        let version = u16::from_be_bytes([frame[4], frame[5]]);
        if version != VERSION {
            return Err(ProtocolError::UnsupportedVersion(version));
        }
        if frame[7] != 0 {
            return Err(ProtocolError::InvalidField("reserved header byte"));
        }
        let kind = MessageType::from_byte(frame[6])?;
        let payload_len = u32::from_be_bytes([frame[8], frame[9], frame[10], frame[11]]) as usize;
        let expected_len = HEADER_LEN
            .checked_add(payload_len)
            .ok_or(ProtocolError::FrameTooLarge(usize::MAX))?;
        if expected_len > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge(expected_len));
        }
        if frame.len() != expected_len {
            return Err(ProtocolError::Truncated);
        }
        let id = u64::from_be_bytes([
            frame[12], frame[13], frame[14], frame[15], frame[16], frame[17], frame[18], frame[19],
        ]);
        decode_message(kind, id, &frame[HEADER_LEN..])
    }

    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, ProtocolError> {
        let mut header = [0; HEADER_LEN];
        reader.read_exact(&mut header).map_err(|error| {
            if error.kind() == io::ErrorKind::UnexpectedEof {
                ProtocolError::Truncated
            } else {
                ProtocolError::Io(error)
            }
        })?;
        let payload_len =
            u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
        let frame_len = HEADER_LEN
            .checked_add(payload_len)
            .ok_or(ProtocolError::FrameTooLarge(usize::MAX))?;
        if frame_len > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge(frame_len));
        }
        let mut frame = Vec::with_capacity(frame_len);
        frame.extend_from_slice(&header);
        frame.resize(frame_len, 0);
        reader
            .read_exact(&mut frame[HEADER_LEN..])
            .map_err(|error| {
                if error.kind() == io::ErrorKind::UnexpectedEof {
                    ProtocolError::Truncated
                } else {
                    ProtocolError::Io(error)
                }
            })?;
        Self::decode_frame(&frame)
    }
}

fn decode_message(kind: MessageType, id: u64, payload: &[u8]) -> Result<Message, ProtocolError> {
    if matches!(
        kind,
        MessageType::Hello | MessageType::Ready | MessageType::Shutdown
    ) && id != 0
    {
        return Err(ProtocolError::InvalidField("control message id"));
    }
    if matches!(
        kind,
        MessageType::Execute
            | MessageType::Response
            | MessageType::Cancel
            | MessageType::Heartbeat
            | MessageType::Error
    ) && id == 0
    {
        return Err(ProtocolError::InvalidField("request message id"));
    }
    let mut cursor = Cursor::new(payload);
    let message = match kind {
        MessageType::Hello => Message::Hello,
        MessageType::Ready => Message::Ready {
            worker_id: cursor.u64()?,
        },
        MessageType::Execute => Message::Execute {
            id,
            request: decode_request(&mut cursor)?,
        },
        MessageType::Response => Message::Response {
            id,
            response: decode_response(&mut cursor)?,
        },
        MessageType::Cancel => Message::Cancel { id },
        MessageType::Shutdown => Message::Shutdown,
        MessageType::Heartbeat => Message::Heartbeat { id },
        MessageType::Error => Message::Error {
            id,
            code: cursor.string()?,
            message: cursor.string()?,
        },
    };
    if cursor.remaining() != 0 {
        return Err(ProtocolError::InvalidField("trailing payload bytes"));
    }
    Ok(message)
}

fn encode_request(payload: &mut Vec<u8>, request: &ExecuteRequest) -> Result<(), ProtocolError> {
    put_string(payload, &request.application)?;
    put_string(payload, &request.method)?;
    put_string(payload, &request.path)?;
    put_u64(payload, request.deadline_ms);
    put_u32(payload, request.headers.len())?;
    for (name, value) in &request.headers {
        put_string(payload, name)?;
        put_string(payload, value)?;
    }
    put_bytes(payload, &request.body)?;
    put_bytes(payload, &request.script)?;
    put_string(payload, &request.context_json)
}

fn encode_response(payload: &mut Vec<u8>, response: &ExecuteResponse) -> Result<(), ProtocolError> {
    put_u16(payload, response.status);
    put_u32(payload, response.headers.len())?;
    for (name, value) in &response.headers {
        put_string(payload, name)?;
        put_string(payload, value)?;
    }
    put_bytes(payload, &response.body)
}

fn decode_request(cursor: &mut Cursor<'_>) -> Result<ExecuteRequest, ProtocolError> {
    let application = cursor.string()?;
    let method = cursor.string()?;
    let path = cursor.string()?;
    let deadline_ms = cursor.u64()?;
    let header_count = cursor.u32()? as usize;
    if header_count > MAX_FRAME_BYTES / 8 {
        return Err(ProtocolError::InvalidField("header count"));
    }
    let mut headers = Vec::with_capacity(header_count);
    for _ in 0..header_count {
        headers.push((cursor.string()?, cursor.string()?));
    }
    Ok(ExecuteRequest {
        application,
        method,
        path,
        deadline_ms,
        headers,
        body: cursor.bytes()?,
        script: cursor.bytes()?,
        context_json: cursor.string()?,
    })
}

fn decode_response(cursor: &mut Cursor<'_>) -> Result<ExecuteResponse, ProtocolError> {
    let status = cursor.u16()?;
    let header_count = cursor.u32()? as usize;
    if header_count > MAX_FRAME_BYTES / 8 {
        return Err(ProtocolError::InvalidField("header count"));
    }
    let mut headers = Vec::with_capacity(header_count);
    for _ in 0..header_count {
        headers.push((cursor.string()?, cursor.string()?));
    }
    Ok(ExecuteResponse {
        status,
        headers,
        body: cursor.bytes()?,
    })
}

fn put_u16(payload: &mut Vec<u8>, value: u16) {
    payload.extend_from_slice(&value.to_be_bytes());
}

fn put_u32(payload: &mut Vec<u8>, value: usize) -> Result<(), ProtocolError> {
    let value = u32::try_from(value).map_err(|_| ProtocolError::InvalidField("length"))?;
    payload.extend_from_slice(&value.to_be_bytes());
    Ok(())
}

fn put_u64(payload: &mut Vec<u8>, value: u64) {
    payload.extend_from_slice(&value.to_be_bytes());
}

fn put_string(payload: &mut Vec<u8>, value: &str) -> Result<(), ProtocolError> {
    put_bytes(payload, value.as_bytes())
}

fn put_bytes(payload: &mut Vec<u8>, value: &[u8]) -> Result<(), ProtocolError> {
    put_u32(payload, value.len())?;
    payload.extend_from_slice(value);
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ProtocolError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(ProtocolError::Truncated)?;
        if end > self.bytes.len() {
            return Err(ProtocolError::Truncated);
        }
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, ProtocolError> {
        let value = self.take(2)?;
        Ok(u16::from_be_bytes([value[0], value[1]]))
    }

    fn u32(&mut self) -> Result<u32, ProtocolError> {
        let value = self.take(4)?;
        Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
    }

    fn u64(&mut self) -> Result<u64, ProtocolError> {
        let value = self.take(8)?;
        Ok(u64::from_be_bytes([
            value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
        ]))
    }

    fn bytes(&mut self) -> Result<Vec<u8>, ProtocolError> {
        let length = self.u32()? as usize;
        Ok(self.take(length)?.to_vec())
    }

    fn string(&mut self) -> Result<String, ProtocolError> {
        String::from_utf8(self.bytes()?).map_err(|_| ProtocolError::InvalidUtf8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor as IoCursor;

    #[test]
    fn execute_round_trips_binary_fields() {
        let message = Message::Execute {
            id: 42,
            request: ExecuteRequest {
                application: "app-a".into(),
                method: "POST".into(),
                path: "/upload".into(),
                deadline_ms: 1_725_000_000_000,
                headers: vec![("content-type".into(), "application/octet-stream".into())],
                body: vec![0, 1, 2, 255],
                script: b"export function POST() { return 'ok'; }".to_vec(),
                context_json: r#"{"method":"POST"}"#.into(),
            },
        };
        let frame = message.encode().expect("message encodes");
        assert_eq!(
            Message::decode_frame(&frame).expect("message decodes"),
            message
        );
    }

    #[test]
    fn response_round_trips_through_reader() {
        let message = Message::Response {
            id: 7,
            response: ExecuteResponse {
                status: 201,
                headers: vec![
                    ("set-cookie".into(), "a=b".into()),
                    ("set-cookie".into(), "c=d".into()),
                ],
                body: b"created".to_vec(),
            },
        };
        let frame = message.encode().expect("message encodes");
        let decoded = Message::read_from(&mut IoCursor::new(frame)).expect("message reads");
        assert_eq!(decoded, message);
    }

    #[test]
    fn invalid_magic_is_rejected() {
        let mut frame = Message::Hello.encode().expect("message encodes");
        frame[0] = b'X';
        assert!(matches!(
            Message::decode_frame(&frame),
            Err(ProtocolError::InvalidMagic(_))
        ));
    }

    #[test]
    fn truncated_payload_is_rejected() {
        let mut frame = Message::Ready { worker_id: 9 }
            .encode()
            .expect("message encodes");
        frame.pop();
        assert!(matches!(
            Message::decode_frame(&frame),
            Err(ProtocolError::Truncated)
        ));
    }

    #[test]
    fn oversized_payload_is_rejected_before_allocation() {
        let mut frame = vec![0; HEADER_LEN];
        frame[..4].copy_from_slice(&MAGIC);
        frame[4..6].copy_from_slice(&VERSION.to_be_bytes());
        frame[6] = MessageType::Hello as u8;
        frame[8..12].copy_from_slice(&((MAX_FRAME_BYTES as u32) + 1).to_be_bytes());
        assert!(matches!(
            Message::decode_frame(&frame),
            Err(ProtocolError::FrameTooLarge(_))
        ));
    }
}
