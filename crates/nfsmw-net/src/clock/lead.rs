//! The input lead (sync-model.md section 5.3).

use nfsmw_sim::TICK_HZ;

use crate::input::MAX_AHEAD_TICKS;
use crate::schema::InputReport;

/// The lead may rise at most once per this, milliseconds.
const RAISE_EVERY_MS: u64 = 250;
/// The lead falls only after earliness stays high this long, milliseconds.
const HIGH_FOR_MS: u64 = 2000;
/// The lead may fall at most once per this, milliseconds.
const LOWER_EVERY_MS: u64 = 2000;
/// Mean earliness, ticks, below which the lead rises.
const LOW_EARLINESS: u8 = 1;
/// Mean earliness, ticks, above which (for 2 s) the lead falls.
const HIGH_EARLINESS: u8 = 3;

/// How many ticks ahead of the estimated server tick the owner predicts and sends its inputs.
///
/// Raised by one tick (at most once per 250 ms) when the server reports a mean earliness under 1 tick or any late or
/// missing input; lowered by one (at most once per 2 s) when the mean earliness stays above 3 ticks for 2 s. Never
/// below 1 tick nor above [`MAX_AHEAD_TICKS`], past which the server drops inputs as too early.
#[derive(Debug, Clone, PartialEq)]
pub struct LeadControl {
    lead: u32,
    last_raise_ms: Option<u64>,
    last_lower_ms: Option<u64>,
    high_since_ms: Option<u64>,
}

impl LeadControl {
    /// A lead of `lead` ticks, clamped to the allowed range.
    pub fn new(lead: u32) -> Self {
        Self { lead: lead.clamp(1, MAX_AHEAD_TICKS), last_raise_ms: None, last_lower_ms: None, high_since_ms: None }
    }

    /// The starting lead for a round trip of `rtt_ms`: half of it in ticks, rounded up, plus the input buffer
    /// (`Tuning::input_buffer_ticks`: 1 on LAN, 2 online).
    pub fn from_rtt(rtt_ms: f64, buffer_ticks: u32) -> Self {
        let one_way = (rtt_ms.max(0.0) / 2.0 * f64::from(TICK_HZ) / 1000.0).ceil();
        let one_way = match one_way.is_finite() {
            true => one_way.min(f64::from(MAX_AHEAD_TICKS)) as u32,
            false => MAX_AHEAD_TICKS,
        };
        Self::new(one_way + buffer_ticks)
    }

    /// The lead in ticks.
    pub fn ticks(&self) -> u32 {
        self.lead
    }

    /// The tick to predict now: `C = S_est + lead`.
    pub fn predicted_tick(&self, server_tick: f64) -> f64 {
        server_tick + f64::from(self.lead)
    }

    /// Take in the server's input report, received at `now_ms`. Returns the change of the lead in ticks (-1, 0 or 1).
    pub fn on_report(&mut self, report: &InputReport, now_ms: u64) -> i32 {
        let received = report.early + report.on_time;
        let short = report.late > 0 || report.missing > 0 || (received > 0 && report.earliness < LOW_EARLINESS);
        if short {
            self.high_since_ms = None;
            return self.raise(now_ms);
        }
        if report.earliness <= HIGH_EARLINESS {
            self.high_since_ms = None;
            return 0;
        }
        let since = *self.high_since_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(since) < HIGH_FOR_MS {
            return 0;
        }
        self.lower(now_ms)
    }

    fn raise(&mut self, now_ms: u64) -> i32 {
        if self.last_raise_ms.is_some_and(|last| now_ms.saturating_sub(last) < RAISE_EVERY_MS) {
            return 0;
        }
        if self.lead >= MAX_AHEAD_TICKS {
            return 0;
        }
        self.lead += 1;
        self.last_raise_ms = Some(now_ms);
        1
    }

    fn lower(&mut self, now_ms: u64) -> i32 {
        if self.last_lower_ms.is_some_and(|last| now_ms.saturating_sub(last) < LOWER_EVERY_MS) {
            return 0;
        }
        if self.lead <= 1 {
            return 0;
        }
        self.lead -= 1;
        self.last_lower_ms = Some(now_ms);
        -1
    }
}
