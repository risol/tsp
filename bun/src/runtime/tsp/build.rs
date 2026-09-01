use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=TSP_VERSION");
    println!("cargo:rerun-if-changed=build.rs");

    if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let manifest_dir = Path::new(&manifest_dir);
        if let Some(git_dir) = git_dir(manifest_dir) {
            for path in ["HEAD", "packed-refs", "refs/tags", "refs/heads"] {
                println!("cargo:rerun-if-changed={}", git_dir.join(path).display());
            }
        }
    }

    let version = std::env::var("TSP_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| normalize_version(&value))
        .or_else(version_from_git)
        .unwrap_or_else(|| "0.0.0-dev".to_string());

    println!("cargo:rustc-env=TSP_VERSION={version}");
}

fn version_from_git() -> Option<String> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")?;
    let value = git_output(
        Path::new(&manifest_dir),
        &[
            "describe", "--tags", "--always", "--dirty", "--match", "v[0-9]*",
        ],
    )?;
    (!value.is_empty()).then(|| normalize_version(&value))
}

fn git_dir(manifest_dir: &Path) -> Option<std::path::PathBuf> {
    let value = git_output(manifest_dir, &["rev-parse", "--git-dir"])?;
    let path = Path::new(&value);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        manifest_dir.join(path)
    })
}

fn git_output(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(manifest_dir)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_string())
}

fn normalize_version(value: &str) -> String {
    let value = value.trim();
    let value = value.strip_prefix("refs/tags/").unwrap_or(value);
    value.strip_prefix('v').unwrap_or(value).to_string()
}
