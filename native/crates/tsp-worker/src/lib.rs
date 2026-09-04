//! Versioned, bounded worker-process framing.
//!
//! Frames contain JSON payloads for debuggability. The length prefix and
//! maximum frame size are owned by this crate so a malformed child cannot
//! allocate unbounded memory in the master.

use std::io::{self, Read, Write};

pub const MAX_FRAME_BYTES: u32 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub kind: u8,
    pub payload: Vec<u8>,
}

pub fn write_frame(writer: &mut impl Write, frame: &Frame) -> io::Result<()> {
    let length = frame
        .payload
        .len()
        .checked_add(1)
        .filter(|length| *length <= MAX_FRAME_BYTES as usize)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "worker frame is too large"))?;
    writer.write_all(&(length as u32).to_be_bytes())?;
    writer.write_all(&[frame.kind])?;
    writer.write_all(&frame.payload)?;
    writer.flush()
}

pub fn read_frame(reader: &mut impl Read) -> io::Result<Option<Frame>> {
    let mut length_bytes = [0; 4];
    match reader.read_exact(&mut length_bytes) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let length = u32::from_be_bytes(length_bytes);
    if !(1..=MAX_FRAME_BYTES).contains(&length) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "worker frame length is invalid",
        ));
    }
    let mut bytes = vec![0; length as usize];
    reader.read_exact(&mut bytes)?;
    Ok(Some(Frame {
        kind: bytes[0],
        payload: bytes[1..].to_vec(),
    }))
}

pub fn json_frame<T: serde::Serialize>(kind: u8, value: &T) -> Result<Frame, serde_json::Error> {
    Ok(Frame {
        kind,
        payload: serde_json::to_vec(value)?,
    })
}

pub fn parse_json<T: serde::de::DeserializeOwned>(frame: &Frame) -> Result<T, serde_json::Error> {
    serde_json::from_slice(&frame.payload)
}

pub mod kind {
    pub const COMMAND: u8 = 1;
    pub const EVENT: u8 = 2;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_round_trip_preserves_kind_and_payload() {
        let frame = Frame {
            kind: kind::COMMAND,
            payload: br#"{"type":"Ping"}"#.to_vec(),
        };
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &frame).unwrap();
        assert_eq!(read_frame(&mut bytes.as_slice()).unwrap(), Some(frame));
    }

    #[test]
    fn oversized_frames_are_rejected_before_write() {
        let frame = Frame {
            kind: kind::COMMAND,
            payload: vec![0; MAX_FRAME_BYTES as usize],
        };
        let error = write_frame(&mut Vec::new(), &frame).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}
