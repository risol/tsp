//! Safe native serving for the configured `public/` directory.

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
/// A URL ending in `/` (including `/`) serves its `index.html` when present.
pub fn load(root: &Path, request_path: &str) -> io::Result<Option<StaticFile>> {
    let Some(relative) = decode_path(request_path) else {
        return Ok(None);
    };

    // Canonicalize the root first. Apart from making the containment check
    // reliable for relative configuration, this also means a missing or
    // unreadable public directory simply behaves like an empty one.
    let Ok(canonical_root) = crate::path::canonicalize(root) else {
        return Ok(None);
    };
    if !canonical_root.is_dir() {
        return Ok(None);
    }

    let mut candidate = canonical_root.clone();
    for segment in relative.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        // Reject both separators. `/` is handled by the split above; `\\`
        // must also be rejected because it is a path separator on Windows
        // even though it is not a URL separator. This keeps the same URL
        // safe on every supported platform, including encoded backslashes.
        if segment == ".." || segment.contains(['\\', '\0']) {
            return Ok(None);
        }
        candidate.push(segment);
    }

    if candidate.is_dir() && (relative == "/" || relative.ends_with('/')) {
        candidate.push("index.html");
    }

    let canonical = match crate::path::canonicalize(&candidate) {
        Ok(path) => path,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !canonical.starts_with(&canonical_root) {
        return Ok(None);
    }
    let metadata = match fs::metadata(&canonical) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() {
        return Ok(None);
    }

    let body = match fs::read(&canonical) {
        Ok(body) => body,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
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
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",
        "wasm" => "application/wasm",
        "webmanifest" => "application/manifest+json",
        "map" => "application/json; charset=utf-8",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_index_and_decoded_file_without_escape() {
        let root = test_root("basic");
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
    fn directory_index_and_common_content_types_are_supported() {
        let root = test_root("directory-index");
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/index.html"), "docs").unwrap();
        std::fs::write(root.join("app.woff2"), b"font").unwrap();
        std::fs::write(root.join("site.webmanifest"), b"{}").unwrap();

        let directory = load(&root, "/docs/").unwrap().unwrap();
        assert_eq!(directory.body, b"docs");
        assert_eq!(directory.content_type, "text/html; charset=utf-8");
        assert_eq!(
            load(&root, "/app.woff2").unwrap().unwrap().content_type,
            "font/woff2"
        );
        assert_eq!(
            load(&root, "/site.webmanifest")
                .unwrap()
                .unwrap()
                .content_type,
            "application/manifest+json"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_posix_and_windows_traversal() {
        let root = test_root("traversal");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("secret.txt"), "secret").unwrap();

        assert!(load(&root, "/../secret.txt").unwrap().is_none());
        assert!(load(&root, "/%2e%2e/secret.txt").unwrap().is_none());
        assert!(load(&root, "/..%5csecret.txt").unwrap().is_none());
        assert!(load(&root, "/%2e%2e%5csecret.txt").unwrap().is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_public_file_falls_through() {
        let root = test_root("missing");
        std::fs::create_dir_all(&root).unwrap();
        assert!(load(&root, "/missing.js").unwrap().is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    fn test_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "tspserver-public-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
