//! Safe native serving for the configured v2 `public/` directory.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct StaticFile {
    pub body: Vec<u8>,
    pub content_type: &'static str,
    pub path: PathBuf,
}

/// Resolve a URL path below `root`. Missing files return `None` so the page
/// router can handle the request. Existing files are canonicalized and must
/// remain below the public root, which blocks traversal and symlink escape.
pub fn load(root: &Path, request_path: &str) -> io::Result<Option<StaticFile>> {
    let Some(relative) = decode_path(request_path) else {
        return Ok(None);
    };
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut candidate = canonical_root.clone();
    for segment in relative.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." || segment.contains('\0') {
            return Ok(None);
        }
        candidate.push(segment);
    }
    if candidate.is_dir() && relative == "/" {
        candidate.push("index.html");
    }
    if !candidate.is_file() {
        return Ok(None);
    }
    let canonical = candidate.canonicalize()?;
    if !canonical.starts_with(&canonical_root) {
        return Ok(None);
    }
    let body = fs::read(&canonical)?;
    let content_type = mime_type(&canonical);
    Ok(Some(StaticFile {
        body,
        content_type,
        path: canonical,
    }))
}

fn decode_path(path: &str) -> Option<String> {
    if !path.starts_with('/') {
        return None;
    }
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return None;
            }
            let hi = hex(bytes[index + 1])?;
            let lo = hex(bytes[index + 2])?;
            out.push((hi << 4) | lo);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_index_and_decoded_file_without_escape() {
        let root = std::env::temp_dir().join(format!("tsp-v2-public-{}", std::process::id()));
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("index.html"), "home").unwrap();
        std::fs::write(root.join("assets/hello world.txt"), "hello").unwrap();
        let index = load(&root, "/").unwrap().unwrap();
        assert_eq!(index.content_type, "text/html; charset=utf-8");
        assert_eq!(index.body, b"home");
        let file = load(&root, "/assets/hello%20world.txt").unwrap().unwrap();
        assert_eq!(file.body, b"hello");
        assert!(load(&root, "/%2e%2e/secret").unwrap().is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_public_file_falls_through() {
        let root =
            std::env::temp_dir().join(format!("tsp-v2-public-missing-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        assert!(load(&root, "/missing.js").unwrap().is_none());
        let _ = std::fs::remove_dir_all(root);
    }
}
