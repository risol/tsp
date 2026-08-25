//! Optional Linux cgroup v2 limits for worker processes.
//!
//! Limits are opt-in through an explicit cgroup root. This avoids silently
//! mutating a system cgroup hierarchy on development machines while still
//! providing a real cgroup v2 implementation in production deployments.

use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct ResourceLimits {
    pub cgroup_root: Option<PathBuf>,
    pub memory_max: Option<u64>,
    pub cpu_max: Option<String>,
    pub pids_max: Option<u64>,
}

impl ResourceLimits {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn enabled(&self) -> bool {
        self.cgroup_root.is_some()
            && (self.memory_max.is_some() || self.cpu_max.is_some() || self.pids_max.is_some())
    }
}

#[derive(Debug)]
pub enum SandboxError {
    Io(std::io::Error),
    UnsupportedPlatform,
    InvalidConfiguration(&'static str),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "worker resource isolation I/O failed: {error}"),
            Self::UnsupportedPlatform => write!(f, "cgroup v2 is only available on Linux"),
            Self::InvalidConfiguration(field) => {
                write!(f, "invalid worker resource limit: {field}")
            }
        }
    }
}

impl std::error::Error for SandboxError {}

impl From<std::io::Error> for SandboxError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub struct CgroupHandle {
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    path: PathBuf,
}

impl CgroupHandle {
    pub fn attach(pid: u32, limits: &ResourceLimits) -> Result<Option<Self>, SandboxError> {
        if !limits.enabled() {
            return Ok(None);
        }
        #[cfg(target_os = "linux")]
        {
            let root = limits
                .cgroup_root
                .as_ref()
                .ok_or(SandboxError::InvalidConfiguration("cgroup_root"))?;
            let path = root.join(format!("tsp-worker-{pid}"));
            std::fs::create_dir_all(&path)?;
            if let Err(error) = (|| {
                if let Some(value) = limits.memory_max {
                    write_limit(&path, "memory.max", value.to_string())?;
                }
                if let Some(value) = &limits.cpu_max {
                    if value.trim().is_empty() {
                        return Err(SandboxError::InvalidConfiguration("cpu_max"));
                    }
                    write_limit(&path, "cpu.max", value.clone())?;
                }
                if let Some(value) = limits.pids_max {
                    write_limit(&path, "pids.max", value.to_string())?;
                }
                std::fs::write(path.join("cgroup.procs"), pid.to_string())?;
                Ok::<(), SandboxError>(())
            })() {
                let _ = std::fs::remove_dir(&path);
                return Err(error);
            }
            return Ok(Some(Self { path }));
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (pid, limits);
            Err(SandboxError::UnsupportedPlatform)
        }
    }
}

#[cfg(target_os = "linux")]
fn write_limit(path: &std::path::Path, name: &str, value: String) -> Result<(), SandboxError> {
    std::fs::write(path.join(name), value)?;
    Ok(())
}

impl Drop for CgroupHandle {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        {
            let _ = std::fs::remove_dir(&self.path);
        }
    }
}
