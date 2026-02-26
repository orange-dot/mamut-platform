use std::time::{SystemTime, UNIX_EPOCH};

/// Returns current UTC timestamp as `(seconds, nanos)` since Unix epoch.
pub fn now_unix_timestamp() -> (i64, i32) {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (duration.as_secs() as i64, duration.subsec_nanos() as i32)
}
