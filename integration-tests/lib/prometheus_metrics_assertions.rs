//! Helpers for querying and asserting on Prometheus metrics and JSON API endpoints
//! exposed by SV2 components during integration tests.

use std::{collections::HashMap, fmt, net::SocketAddr};

/// Fetch the raw Prometheus text-format metrics from a component's `/metrics` endpoint.
/// Uses `spawn_blocking` to avoid blocking the tokio runtime with synchronous HTTP calls.
pub async fn fetch_metrics(monitoring_addr: SocketAddr) -> String {
    let url = format!("http://{}/metrics", monitoring_addr);
    tokio::task::spawn_blocking(move || {
        let bytes = crate::utils::http::make_get_request(&url, 5);
        String::from_utf8(bytes).expect("metrics response should be valid UTF-8")
    })
    .await
    .expect("spawn_blocking for fetch_metrics panicked")
}

/// Fetch the JSON body from a component's API endpoint (e.g. `/api/v1/health`).
/// Uses `spawn_blocking` to avoid blocking the tokio runtime with synchronous HTTP calls.
pub async fn fetch_api(monitoring_addr: SocketAddr, path: &str) -> String {
    let url = format!("http://{}{}", monitoring_addr, path);
    tokio::task::spawn_blocking(move || {
        let bytes = crate::utils::http::make_get_request(&url, 5);
        String::from_utf8(bytes).expect("api response should be valid UTF-8")
    })
    .await
    .expect("spawn_blocking for fetch_api panicked")
}

/// A Prometheus metric selector: a metric name plus an optional set of label matchers.
///
/// Label matching is order-independent — the selector matches any exposition line
/// whose label set is a superset of the requested labels. A selector with no labels
/// matches any line for that metric (bare or labeled).
///
/// # Examples
///
/// ```
/// # use integration_tests_sv2::prometheus_metrics_assertions::Metric;
/// // Bare name (implicit via From<&str>):
/// let _: Metric = "sv2_clients_total".into();
///
/// // Specific labeled series:
/// let _ = Metric::with_labels(
///     "sv2_server_shares_accepted_total",
///     &[("channel_id", "1"), ("user_identity", "user1")],
/// );
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Metric<'a> {
    pub name: &'a str,
    pub labels: &'a [(&'a str, &'a str)],
}

impl<'a> Metric<'a> {
    /// Create a selector for a metric by bare name (matches any labels).
    pub const fn new(name: &'a str) -> Self {
        Self { name, labels: &[] }
    }

    /// Create a selector with specific label matchers. Matches lines whose label
    /// set is a superset of `labels`, regardless of label ordering.
    pub const fn with_labels(name: &'a str, labels: &'a [(&'a str, &'a str)]) -> Self {
        Self { name, labels }
    }

    /// Try to match a single Prometheus exposition line. Returns the parsed value
    /// if the line matches this selector, otherwise `None`.
    fn match_line(&self, line: &str) -> Option<f64> {
        let rest = line.strip_prefix(self.name)?;
        // The name must be a complete token: next char is whitespace, '{', or EOL.
        // This prevents e.g. `sv2_clients_total_extra` from matching `sv2_clients_total`.
        let is_labeled = rest.starts_with('{');
        let is_bare = rest.chars().next().is_none_or(|c| c.is_ascii_whitespace());
        if !is_labeled && !is_bare {
            return None;
        }

        // Parse the labels (if any) and the value portion.
        let (line_labels, value_part) = if is_labeled {
            let inner = rest.strip_prefix('{')?;
            let (block, after) = inner.split_once('}')?;
            (parse_label_block(block), after)
        } else {
            (HashMap::new(), rest)
        };

        // Selector labels must all appear on the line with matching values.
        for (k, v) in self.labels {
            if line_labels.get(*k).map(String::as_str) != Some(*v) {
                return None;
            }
        }

        value_part.split_whitespace().next()?.parse().ok()
    }
}

impl<'a> From<&'a str> for Metric<'a> {
    fn from(name: &'a str) -> Self {
        Metric::new(name)
    }
}

impl fmt::Display for Metric<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)?;
        if !self.labels.is_empty() {
            f.write_str("{")?;
            for (i, (k, v)) in self.labels.iter().enumerate() {
                if i > 0 {
                    f.write_str(",")?;
                }
                write!(f, "{}=\"{}\"", k, v)?;
            }
            f.write_str("}")?;
        }
        Ok(())
    }
}

/// Parse the inside of a Prometheus label block like `k1="v1",k2="v2"` into a map.
/// Supports the subset emitted by the `prometheus` crate: simple `k="v"` pairs with
/// no escape sequences in values (sufficient for our metrics).
fn parse_label_block(block: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let block = block.trim();
    if block.is_empty() {
        return out;
    }
    for pair in block.split(',') {
        let pair = pair.trim();
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        let v = v.trim().trim_start_matches('"').trim_end_matches('"');
        out.insert(k.trim().to_string(), v.to_string());
    }
    out
}

/// Parse a specific metric value from Prometheus text format.
/// Returns `None` if no line matches the selector.
pub(crate) fn parse_metric_value<'a, M: Into<Metric<'a>>>(
    metrics_text: &str,
    metric: M,
) -> Option<f64> {
    let metric = metric.into();
    for line in metrics_text.lines() {
        if line.starts_with('#') {
            continue;
        }
        if let Some(v) = metric.match_line(line) {
            return Some(v);
        }
    }
    None
}

/// Assert that a metric is present and its value satisfies the given predicate.
pub(crate) fn assert_metric<'a, M, F>(
    metrics_text: &str,
    metric: M,
    predicate: F,
    description: &str,
) where
    M: Into<Metric<'a>>,
    F: Fn(f64) -> bool,
{
    let metric = metric.into();
    match parse_metric_value(metrics_text, metric) {
        Some(v) => {
            assert!(
                predicate(v),
                "Metric '{}' has value {} but expected: {}",
                metric,
                v,
                description
            );
        }
        None => {
            panic!(
                "Metric '{}' not found in metrics output. Expected: {}",
                metric, description
            );
        }
    }
}

/// Assert that a metric is present with a value >= the given minimum.
pub fn assert_metric_gte<'a, M: Into<Metric<'a>>>(metrics_text: &str, metric: M, min: f64) {
    assert_metric(metrics_text, metric, |v| v >= min, &format!(">= {}", min));
}

/// Assert that a metric is present with the exact given value.
pub fn assert_metric_eq<'a, M: Into<Metric<'a>>>(metrics_text: &str, metric: M, expected: f64) {
    assert_metric(
        metrics_text,
        metric,
        |v| (v - expected).abs() < f64::EPSILON,
        &format!("== {}", expected),
    );
}

/// Assert that no exposition line matches the selector.
///
/// For a bare-name selector (`Metric::new("name")` or `"name".into()`), this means
/// the metric name does not appear at all. For a labeled selector, it means no line
/// with matching labels exists — other series for the same metric name are allowed.
pub fn assert_metric_not_present<'a, M: Into<Metric<'a>>>(metrics_text: &str, metric: M) {
    let metric = metric.into();
    for line in metrics_text.lines() {
        if line.starts_with('#') {
            continue;
        }
        if metric.match_line(line).is_some() {
            panic!(
                "Metric '{}' was found in metrics output but was expected to be absent. Line: {}",
                metric, line
            );
        }
    }
}

/// Assert that at least one exposition line matches the selector.
pub fn assert_metric_present<'a, M: Into<Metric<'a>>>(metrics_text: &str, metric: M) {
    let metric = metric.into();
    for line in metrics_text.lines() {
        if line.starts_with('#') {
            continue;
        }
        if metric.match_line(line).is_some() {
            return;
        }
    }
    panic!(
        "Metric '{}' was expected to be present but was not found in metrics output",
        metric
    );
}

/// Poll `/metrics` until a line matching `metric` has value >= `min`, or panic after
/// `timeout`. Polls every 100ms to react quickly while tolerating cache refresh jitter.
///
/// Returns the full metrics text from the successful scrape so callers can make additional
/// assertions without a second fetch.
pub async fn poll_until_metric_gte<'a, M: Into<Metric<'a>>>(
    monitoring_addr: SocketAddr,
    metric: M,
    min: f64,
    timeout: std::time::Duration,
) -> String {
    let metric = metric.into();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let metrics = fetch_metrics(monitoring_addr).await;
        if let Some(v) = parse_metric_value(&metrics, metric) {
            if v >= min {
                return metrics;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            panic!(
                "Metric '{}' never reached >= {} within {:?}. Last /metrics response:\n{}",
                metric, min, timeout, metrics
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// Assert that the `/api/v1/health` endpoint returns a response containing `"status":"ok"`.
pub async fn assert_api_health(monitoring_addr: SocketAddr) {
    let body = fetch_api(monitoring_addr, "/api/v1/health").await;
    assert!(
        body.contains("\"status\":\"ok\""),
        "Health endpoint should return ok status, got: {}",
        body
    );
}

/// Assert that the uptime metric is present and positive.
pub fn assert_uptime(metrics_text: &str) {
    assert_metric(
        metrics_text,
        "sv2_uptime_seconds",
        |v| v >= 0.0,
        ">= 0.0 (uptime should be non-negative)",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_METRICS: &str = r#"# HELP sv2_uptime_seconds Server uptime in seconds
# TYPE sv2_uptime_seconds gauge
sv2_uptime_seconds 42
# HELP sv2_clients_total Total number of connected clients
# TYPE sv2_clients_total gauge
sv2_clients_total 3
# HELP sv2_server_channels Number of server channels by type
# TYPE sv2_server_channels gauge
sv2_server_channels{channel_type="extended"} 1
sv2_server_channels{channel_type="standard"} 0
# HELP sv2_client_shares_accepted_total Per-channel accepted shares
# TYPE sv2_client_shares_accepted_total gauge
sv2_client_shares_accepted_total{channel_id="1",user_identity="user1"} 5
"#;

    #[test]
    fn test_parse_simple_metric() {
        assert_eq!(
            parse_metric_value(SAMPLE_METRICS, "sv2_uptime_seconds"),
            Some(42.0)
        );
        assert_eq!(
            parse_metric_value(SAMPLE_METRICS, "sv2_clients_total"),
            Some(3.0)
        );
    }

    #[test]
    fn test_parse_labeled_metric() {
        assert_eq!(
            parse_metric_value(
                SAMPLE_METRICS,
                Metric::with_labels("sv2_server_channels", &[("channel_type", "extended")])
            ),
            Some(1.0)
        );
        assert_eq!(
            parse_metric_value(
                SAMPLE_METRICS,
                Metric::with_labels("sv2_server_channels", &[("channel_type", "standard")])
            ),
            Some(0.0)
        );
    }

    #[test]
    fn test_label_order_independence() {
        // Selector requesting labels in opposite order to the exposition line
        // must still match — the prometheus crate emits in BTreeMap order today,
        // but tests should not silently break if that ever changes.
        assert_eq!(
            parse_metric_value(
                SAMPLE_METRICS,
                Metric::with_labels(
                    "sv2_client_shares_accepted_total",
                    &[("user_identity", "user1"), ("channel_id", "1")],
                )
            ),
            Some(5.0)
        );
    }

    #[test]
    fn test_label_subset_match() {
        // Querying only a subset of labels still matches.
        assert_eq!(
            parse_metric_value(
                SAMPLE_METRICS,
                Metric::with_labels("sv2_client_shares_accepted_total", &[("channel_id", "1")])
            ),
            Some(5.0)
        );
    }

    #[test]
    fn test_label_mismatch_returns_none() {
        assert_eq!(
            parse_metric_value(
                SAMPLE_METRICS,
                Metric::with_labels("sv2_server_channels", &[("channel_type", "nonexistent")])
            ),
            None
        );
    }

    #[test]
    fn test_bare_selector_matches_labeled_line() {
        // A bare-name selector matches any series for that metric (returns the
        // first one found).
        assert_eq!(
            parse_metric_value(SAMPLE_METRICS, "sv2_server_channels"),
            Some(1.0)
        );
    }

    #[test]
    fn test_parse_missing_metric() {
        assert_eq!(
            parse_metric_value(SAMPLE_METRICS, "nonexistent_metric"),
            None
        );
    }

    #[test]
    fn test_assert_metric_gte() {
        assert_metric_gte(SAMPLE_METRICS, "sv2_clients_total", 1.0);
        assert_metric_gte(SAMPLE_METRICS, "sv2_clients_total", 3.0);
    }

    #[test]
    fn test_assert_metric_eq() {
        assert_metric_eq(SAMPLE_METRICS, "sv2_uptime_seconds", 42.0);
    }

    #[test]
    fn test_assert_metric_not_present() {
        assert_metric_not_present(SAMPLE_METRICS, "nonexistent_metric");
    }

    #[test]
    #[should_panic(expected = "was found in metrics output")]
    fn test_assert_metric_not_present_panics() {
        assert_metric_not_present(SAMPLE_METRICS, "sv2_clients_total");
    }

    #[test]
    fn test_assert_metric_present() {
        assert_metric_present(SAMPLE_METRICS, "sv2_clients_total");
        assert_metric_present(SAMPLE_METRICS, "sv2_server_channels");
    }

    #[test]
    #[should_panic(expected = "was expected to be present")]
    fn test_assert_metric_present_panics() {
        assert_metric_present(SAMPLE_METRICS, "nonexistent_metric");
    }

    #[test]
    fn test_assert_uptime() {
        assert_uptime(SAMPLE_METRICS);
    }

    #[test]
    fn test_no_false_prefix_match() {
        // sv2_clients_total should not match sv2_clients_total_extra
        let metrics = "sv2_clients_total_extra 99\n";
        assert_metric_not_present(metrics, "sv2_clients_total");
    }
}
