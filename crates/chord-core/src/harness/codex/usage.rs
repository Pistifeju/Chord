//! Codex's subscription usage: the rate limit windows of `account/rateLimits/read` as usage windows.

use serde_json::Value;

use crate::harness::UsageWindow;

/// The primary and secondary windows of a rate limit snapshot, shortest first; none for an API key, which has no snapshot.
pub(super) fn windows(snapshot: &Value) -> Vec<UsageWindow> {
    let mut windows: Vec<(u64, UsageWindow)> = [&snapshot["primary"], &snapshot["secondary"]]
        .into_iter()
        .filter_map(|window| {
            let minutes = window["windowDurationMins"].as_u64();
            let resets_at = window["resetsAt"].as_f64().map(|seconds| seconds * 1000.0);
            let used = UsageWindow::used(
                &minutes.map_or_else(|| "usage".into(), name),
                window["usedPercent"].as_f64()?,
                resets_at,
            );
            Some((minutes.unwrap_or(u64::MAX), used))
        })
        .collect();
    windows.sort_by_key(|(minutes, _)| *minutes);
    windows.into_iter().map(|(_, window)| window).collect()
}

// A window's length as the bar names it: "5h", "week", "month".
fn name(minutes: u64) -> String {
    const HOUR: u64 = 60;
    const DAY: u64 = 24 * HOUR;
    match minutes {
        m if m == 7 * DAY => "week".into(),
        m if (28 * DAY..=31 * DAY).contains(&m) => "month".into(),
        m if m % DAY == 0 => format!("{}d", m / DAY),
        m if m % HOUR == 0 => format!("{}h", m / HOUR),
        m => format!("{m}m"),
    }
}
