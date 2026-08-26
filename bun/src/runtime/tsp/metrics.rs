//! Process-local metrics for the native TSP v2 host.
//!
//! The counters are deliberately host-owned and generation-independent. A
//! page reload must never reset operational visibility, and the bridge does
//! not need to retain a JavaScript object just to increment a counter.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct Metrics {
    requests_total: AtomicU64,
    active_requests: AtomicU64,
    duration_ms_sum: AtomicU64,
    duration_samples: AtomicU64,
    responses_2xx: AtomicU64,
    responses_4xx: AtomicU64,
    responses_5xx: AtomicU64,
    timeouts_total: AtomicU64,
    cancellations_total: AtomicU64,
    reloads_total: AtomicU64,
}

impl Metrics {
    pub fn record_request(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_response(&self, status_line: &str) {
        let target = if status_line.starts_with("HTTP/1.1 2") {
            &self.responses_2xx
        } else if status_line.starts_with("HTTP/1.1 4") {
            &self.responses_4xx
        } else if status_line.starts_with("HTTP/1.1 5") {
            &self.responses_5xx
        } else {
            return;
        };
        target.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_timeout(&self) {
        self.timeouts_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn record_cancellation(&self) {
        self.cancellations_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn record_reload(&self) {
        self.reloads_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_duration(&self, duration_ms: u64) {
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
        self.duration_ms_sum
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.duration_samples.fetch_add(1, Ordering::Relaxed);
    }

    pub fn prometheus(&self) -> String {
        let mut out = String::new();
        metric(
            &mut out,
            "tsp_requests_total",
            "Total HTTP requests",
            self.requests_total.load(Ordering::Relaxed),
        );
        gauge(
            &mut out,
            "tsp_active_requests",
            "Requests currently being dispatched",
            self.active_requests.load(Ordering::Relaxed),
        );
        metric(
            &mut out,
            "tsp_request_duration_ms_sum",
            "Sum of request durations in milliseconds",
            self.duration_ms_sum.load(Ordering::Relaxed),
        );
        metric(
            &mut out,
            "tsp_request_duration_ms_count",
            "Number of measured request durations",
            self.duration_samples.load(Ordering::Relaxed),
        );
        metric(
            &mut out,
            "tsp_responses_2xx_total",
            "Responses with a 2xx status",
            self.responses_2xx.load(Ordering::Relaxed),
        );
        metric(
            &mut out,
            "tsp_responses_4xx_total",
            "Responses with a 4xx status",
            self.responses_4xx.load(Ordering::Relaxed),
        );
        metric(
            &mut out,
            "tsp_responses_5xx_total",
            "Responses with a 5xx status",
            self.responses_5xx.load(Ordering::Relaxed),
        );
        metric(
            &mut out,
            "tsp_request_timeouts_total",
            "Requests terminated by the timeout watchdog",
            self.timeouts_total.load(Ordering::Relaxed),
        );
        metric(
            &mut out,
            "tsp_request_cancellations_total",
            "Requests cancelled by disconnect or shutdown",
            self.cancellations_total.load(Ordering::Relaxed),
        );
        metric(
            &mut out,
            "tsp_reload_total",
            "Published route generations after reload",
            self.reloads_total.load(Ordering::Relaxed),
        );
        out
    }
}

fn metric(out: &mut String, name: &str, help: &str, value: u64) {
    out.push_str("# HELP ");
    out.push_str(name);
    out.push(' ');
    out.push_str(help);
    out.push_str("\n# TYPE ");
    out.push_str(name);
    out.push_str(" counter\n");
    out.push_str(name);
    out.push(' ');
    out.push_str(&value.to_string());
    out.push('\n');
}

fn gauge(out: &mut String, name: &str, help: &str, value: u64) {
    out.push_str("# HELP ");
    out.push_str(name);
    out.push(' ');
    out.push_str(help);
    out.push_str("\n# TYPE ");
    out.push_str(name);
    out.push_str(" gauge\n");
    out.push_str(name);
    out.push(' ');
    out.push_str(&value.to_string());
    out.push('\n');
}

static GLOBAL: Metrics = Metrics {
    requests_total: AtomicU64::new(0),
    active_requests: AtomicU64::new(0),
    duration_ms_sum: AtomicU64::new(0),
    duration_samples: AtomicU64::new(0),
    responses_2xx: AtomicU64::new(0),
    responses_4xx: AtomicU64::new(0),
    responses_5xx: AtomicU64::new(0),
    timeouts_total: AtomicU64::new(0),
    cancellations_total: AtomicU64::new(0),
    reloads_total: AtomicU64::new(0),
};

pub fn global() -> &'static Metrics {
    &GLOBAL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prometheus_output_is_stable_and_typed() {
        let metrics = Metrics::default();
        metrics.record_request();
        metrics.record_response("HTTP/1.1 200 OK");
        metrics.record_duration(7);
        metrics.record_response("HTTP/1.1 500 Internal Server Error");
        let text = metrics.prometheus();
        assert!(text.contains("# TYPE tsp_requests_total counter\ntsp_requests_total 1"));
        assert!(text.contains("tsp_responses_2xx_total 1"));
        assert!(text.contains("tsp_responses_5xx_total 1"));
        assert!(text.contains("tsp_active_requests 0"));
        assert!(text.contains("tsp_request_duration_ms_sum 7"));
    }
}
