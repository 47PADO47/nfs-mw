//! `session.json` of a pursuit trace: the build, the settings, the reference place and heat, and the counters.
use std::sync::atomic::Ordering;

use serde::Serialize;
use serde_json::Value;

use super::writer::{Counters, MAX_BYTES};

/// The sample rate, Hz.
pub(super) const SAMPLE_HZ: u32 = 20;

#[derive(Clone, Debug, Default, Serialize)]
pub(super) struct Capture {
    pub records_saved: Value,
    pub bytes_saved: u64,
    pub max_bytes: u64,
    pub disk_dropped: u64,
    pub queue_dropped: u64,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct Session {
    pub schema: u32,
    pub source: &'static str,
    pub created_utc: String,
    pub executable: Value,
    pub remake: Value,
    pub options: Value,
    pub settings: Value,
    /// The bounty place jumped to and the heat used, once a reference jump has landed.
    pub reference: Option<Value>,
    pub schema_notes: Value,
    pub capture: Capture,
    pub stop_reason: Option<String>,
}

impl Session {
    pub fn new(created_utc: String, game_dir: String, settings: Value) -> Session {
        let exe = std::env::current_exe().map(|p| p.display().to_string()).ok();
        Session {
            schema: 1,
            source: "remake",
            created_utc,
            executable: serde_json::json!({"path": exe, "profile": "remake"}),
            remake: serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "git_revision": option_env!("NFSMW_BUILD_REVISION").unwrap_or("unknown"),
                "git_dirty": option_env!("NFSMW_BUILD_DIRTY").map(|d| d == "1"),
                "debug_build": cfg!(debug_assertions),
            }),
            options: serde_json::json!({"game_dir": game_dir, "sample_hz": SAMPLE_HZ, "max_mb": MAX_BYTES >> 20,
                "fixture": false}),
            settings,
            reference: None,
            schema_notes: serde_json::json!({
                "space": "physics space as the original's: x right, y up, z forward; metres, m/s",
                "time_ms": "monotonic milliseconds since the session started; sim_time stops while paused",
                "ids": "cop id 'remake-<entity>:1': entity ids are never reused, so the generation is always 1",
                "nulls": "ai, pursuit_status, drivable_to_nav and nav have no remake equivalent",
                "goal": "the remake goal kind name, not the original's goal hash",
                "remake": "remake-only fields are under 'remake' in samples, cops and events",
                "events": "found by comparing 20 Hz snapshots; unspawn reasons are inferred",
            }),
            capture: Capture::default(),
            stop_reason: None,
        }
    }

    /// Fills the counters at the end of the session.
    pub fn finish(&mut self, counters: &Counters) {
        let samples = counters.samples.load(Ordering::Relaxed);
        let events = counters.events.load(Ordering::Relaxed);
        self.capture = Capture {
            records_saved: serde_json::json!({"samples": samples, "events": events}),
            bytes_saved: counters.bytes.load(Ordering::Relaxed),
            max_bytes: MAX_BYTES,
            disk_dropped: counters.over_budget.load(Ordering::Relaxed),
            queue_dropped: counters.dropped.load(Ordering::Relaxed),
            errors: counters.error().into_iter().collect(),
        };
        self.stop_reason.get_or_insert_with(|| "stopped".into());
    }
}

/// The civil date of a day count since 1970-01-01 (proleptic Gregorian calendar).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = ((mp + 2) % 12 + 1) as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// `(folder stamp, ISO 8601)` of Unix time `secs`: `20261102T201635Z` and `2026-11-02T20:16:35Z`.
pub(super) fn utc(secs: u64) -> (String, String) {
    let (year, month, day) = civil((secs / 86_400) as i64);
    let rest = secs % 86_400;
    let (h, m, s) = (rest / 3600, rest / 60 % 60, rest % 60);
    (
        format!("{year:04}{month:02}{day:02}T{h:02}{m:02}{s:02}Z"),
        format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_stamps_follow_the_calendar() {
        assert_eq!(utc(0).0, "19700101T000000Z");
        assert_eq!(utc(951_782_400).1, "2000-02-29T00:00:00Z");
        assert_eq!(utc(1_792_786_595).0, "20261023T201635Z");
    }
}
