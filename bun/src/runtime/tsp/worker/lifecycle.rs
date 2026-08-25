//! Worker lifetime and recycling policy.
//!
//! Recycling is deliberately a process operation. A worker is never reused
//! after it has crossed an operator-selected request, age, or memory limit;
//! the next request gets a freshly initialized embedded VM.

use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct RecyclePolicy {
    pub max_requests: Option<u64>,
    pub max_age: Option<Duration>,
    pub max_memory_bytes: Option<u64>,
}

impl RecyclePolicy {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn is_enabled(&self) -> bool {
        self.max_requests.is_some() || self.max_age.is_some() || self.max_memory_bytes.is_some()
    }
}

pub fn should_recycle(
    policy: &RecyclePolicy,
    uptime: Duration,
    completed_requests: u64,
    resident_memory_bytes: Option<u64>,
) -> bool {
    policy
        .max_requests
        .is_some_and(|limit| limit > 0 && completed_requests >= limit)
        || policy
            .max_age
            .is_some_and(|limit| !limit.is_zero() && uptime >= limit)
        || policy.max_memory_bytes.is_some_and(|limit| {
            limit > 0 && resident_memory_bytes.is_some_and(|memory| memory >= limit)
        })
}

/// Return the resident set size for a Unix worker where procfs is available.
/// Other platforms intentionally report `None`; request and age limits still
/// work there without pretending that a portable RSS API exists.
#[cfg(target_os = "linux")]
pub fn resident_memory_bytes(pid: u32) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/statm")).ok()?;
    let pages = text.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    // Linux exposes statm in pages. The TSP runtime targets the standard
    // 4KiB-page Linux builds; if a future target needs another page size it
    // can replace this with a platform-specific sysconf adapter.
    Some(pages.saturating_mul(4096))
}

#[cfg(not(target_os = "linux"))]
pub fn resident_memory_bytes(_pid: u32) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_limit_recycles_at_boundary() {
        let policy = RecyclePolicy {
            max_requests: Some(3),
            ..Default::default()
        };
        assert!(!should_recycle(&policy, Duration::ZERO, 2, None));
        assert!(should_recycle(&policy, Duration::ZERO, 3, None));
    }

    #[test]
    fn age_and_memory_limits_are_independent() {
        let policy = RecyclePolicy {
            max_age: Some(Duration::from_secs(2)),
            max_memory_bytes: Some(100),
            ..Default::default()
        };
        assert!(should_recycle(&policy, Duration::from_secs(2), 0, None));
        assert!(should_recycle(&policy, Duration::ZERO, 0, Some(100)));
    }
}
