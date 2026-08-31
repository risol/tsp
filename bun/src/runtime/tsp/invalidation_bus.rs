//! Portable cross-process invalidation transport for multi-worker hosts.
//!
//! The bus carries paths only. Each worker owns its own RouteTable,
//! PageRegistry, and generation state; no JS or generation object crosses the
//! process boundary. An append-only file keeps the transport available on
//! Windows and Unix without introducing a platform-specific socket backend.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct InvalidationBus {
    path: PathBuf,
    reader: File,
    cursor: u64,
}

impl InvalidationBus {
    pub fn open(path: PathBuf) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let reader = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;
        let cursor = reader.metadata()?.len();
        Ok(Self {
            path,
            reader,
            cursor,
        })
    }

    pub fn publish(&mut self, paths: &[PathBuf]) -> io::Result<()> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut writer = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        for path in paths {
            let text = path.to_string_lossy();
            if text.contains('\n') || text.contains('\r') {
                continue;
            }
            writeln!(writer, "{text}")?;
        }
        writer.flush()?;
        self.cursor = writer.metadata()?.len();
        Ok(())
    }

    pub fn read_since(&mut self) -> io::Result<Vec<PathBuf>> {
        let len = self.reader.metadata()?.len();
        if len <= self.cursor {
            return Ok(Vec::new());
        }
        self.reader.seek(SeekFrom::Start(self.cursor))?;
        let mut bytes = Vec::new();
        self.reader.read_to_end(&mut bytes)?;
        let complete_len = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        if complete_len == 0 {
            return Ok(Vec::new());
        }
        self.cursor += complete_len as u64;
        let text = String::from_utf8_lossy(&bytes[..complete_len]);
        Ok(text
            .lines()
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workers_exchange_paths_without_sharing_runtime_state() {
        let path =
            std::env::temp_dir().join(format!("tspserver-invalidation-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut first = InvalidationBus::open(path.clone()).unwrap();
        let mut second = InvalidationBus::open(path.clone()).unwrap();
        first
            .publish(&[
                PathBuf::from("routes/index.tsp"),
                PathBuf::from("routes/shared.tsx"),
            ])
            .unwrap();
        assert!(first.read_since().unwrap().is_empty());
        assert_eq!(
            second.read_since().unwrap(),
            vec![
                PathBuf::from("routes/index.tsp"),
                PathBuf::from("routes/shared.tsx")
            ]
        );
        assert!(second.read_since().unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }
}
