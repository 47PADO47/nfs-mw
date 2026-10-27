//! What the two spark variants (`spark_variant`, `xenon_variant`) share: how strong a collision is, per channel.
//!
//! A profile has two channels: **hit** (the burst of an impact) and **scrape** (the continuing sparks of a slide). Each has a
//! gate, the two numbers that turn the contact's speed into a strength `s` from 0 to 1, and a list of what is played
//! (layers or sparks). The gate replaces the game's own intensity curve while the variant is chosen; the game's triggers (a
//! contact must close or slide to emit at all) stay as they are.
//!
//! ```toml
//! [hit]
//! closing_speed_full = 22.08   # m/s: the closing speed at which the burst is at full strength
//! intensity_floor = 0.153      # 0 to 1: the strength at no closing speed
//!
//! [scrape]
//! speed_start = 0.73           # m/s of sliding speed below which nothing is thrown
//! speed_full = 1.37            # m/s from which the rate is the full one
//! ```

use super::ContentError;

/// The fastest closing or sliding speed a gate names, m/s.
const MAX_SPEED: f32 = 1000.0;

/// The strength of an impact from its closing speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitGate {
    /// The closing speed (m/s) at which the burst is at full strength.
    pub closing_speed_full: f32,
    /// The strength at no closing speed, 0 to 1.
    pub intensity_floor: f32,
}

impl HitGate {
    /// `floor + (1 - floor) * clamp(v / full, 0, 1)` for a closing speed `v`.
    pub fn strength(&self, closing: f32) -> f32 {
        if !closing.is_finite() {
            return 0.0;
        }
        let share = (closing / self.closing_speed_full).clamp(0.0, 1.0);
        self.intensity_floor + (1.0 - self.intensity_floor) * share
    }
}

/// The strength of a scrape from its sliding speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrapeGate {
    /// The sliding speed (m/s) below which nothing is thrown.
    pub speed_start: f32,
    /// The sliding speed (m/s) from which the rate is the full one.
    pub speed_full: f32,
}

impl ScrapeGate {
    /// `clamp((u - start) / (full - start), 0, 1)` for a sliding speed `u`.
    pub fn strength(&self, sliding: f32) -> f32 {
        if !sliding.is_finite() {
            return 0.0;
        }
        ((sliding - self.speed_start) / (self.speed_full - self.speed_start)).clamp(0.0, 1.0)
    }
}

/// The impact of a profile: its gate and what it plays. The gate is there whenever `items` is not empty.
#[derive(Debug, Clone, PartialEq)]
pub struct HitChannel<T> {
    pub gate: Option<HitGate>,
    pub items: Vec<T>,
}

/// The scrape of a profile: its gate and what it plays. The gate is there whenever `items` is not empty.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrapeChannel<T> {
    pub gate: Option<ScrapeGate>,
    pub items: Vec<T>,
}

// A channel with nothing in it is the same as no channel, whatever its gate says (`Default` needs no `T: Default`).
impl<T> Default for HitChannel<T> {
    fn default() -> Self {
        Self { gate: None, items: Vec::new() }
    }
}

impl<T> Default for ScrapeChannel<T> {
    fn default() -> Self {
        Self { gate: None, items: Vec::new() }
    }
}

impl<T> HitChannel<T> {
    /// The strength of an impact at `closing` m/s, or `None` when the channel plays nothing (the install's sparks stay).
    pub fn strength(&self, closing: f32) -> Option<f32> {
        let gate = self.gate.filter(|_| !self.items.is_empty())?;
        Some(gate.strength(closing))
    }
}

impl<T> ScrapeChannel<T> {
    /// The strength of a scrape at `sliding` m/s, or `None` when the channel plays nothing.
    pub fn strength(&self, sliding: f32) -> Option<f32> {
        let gate = self.gate.filter(|_| !self.items.is_empty())?;
        Some(gate.strength(sliding))
    }
}

/// Check the gate of an impact. `needed` is true when the channel has something to play: then both numbers must be given.
pub(super) fn hit_gate(
    closing_speed_full: Option<f32>,
    intensity_floor: Option<f32>,
    needed: bool,
) -> Result<Option<HitGate>, ContentError> {
    if !needed && closing_speed_full.is_none() && intensity_floor.is_none() {
        return Ok(None);
    }
    let full =
        closing_speed_full.ok_or(ContentError::Field { field: "hit.closing_speed_full", reason: "is needed" })?;
    let floor = intensity_floor.ok_or(ContentError::Field { field: "hit.intensity_floor", reason: "is needed" })?;
    if !full.is_finite() || full <= 0.0 || full > MAX_SPEED {
        return Err(ContentError::Field {
            field: "hit.closing_speed_full",
            reason: "a speed above 0 and up to 1000 m/s",
        });
    }
    if !floor.is_finite() || !(0.0..=1.0).contains(&floor) {
        return Err(ContentError::Field { field: "hit.intensity_floor", reason: "a number from 0 to 1" });
    }
    Ok(Some(HitGate { closing_speed_full: full, intensity_floor: floor }))
}

/// Check the gate of a scrape. `needed` is true when the channel has something to play.
pub(super) fn scrape_gate(
    speed_start: Option<f32>,
    speed_full: Option<f32>,
    needed: bool,
) -> Result<Option<ScrapeGate>, ContentError> {
    if !needed && speed_start.is_none() && speed_full.is_none() {
        return Ok(None);
    }
    let start = speed_start.ok_or(ContentError::Field { field: "scrape.speed_start", reason: "is needed" })?;
    let full = speed_full.ok_or(ContentError::Field { field: "scrape.speed_full", reason: "is needed" })?;
    if !start.is_finite() || start < 0.0 || start > MAX_SPEED {
        return Err(ContentError::Field { field: "scrape.speed_start", reason: "a speed from 0 to 1000 m/s" });
    }
    if !full.is_finite() || full <= start || full > MAX_SPEED {
        return Err(ContentError::Field {
            field: "scrape.speed_full",
            reason: "a speed above speed_start, up to 1000 m/s",
        });
    }
    Ok(Some(ScrapeGate { speed_start: start, speed_full: full }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HIT: HitGate = HitGate { closing_speed_full: 20.0, intensity_floor: 0.2 };
    const SCRAPE: ScrapeGate = ScrapeGate { speed_start: 1.0, speed_full: 3.0 };

    #[test]
    fn an_impact_starts_at_the_floor_and_reaches_full_strength_at_the_full_speed() {
        assert_eq!(HIT.strength(0.0), 0.2);
        assert!((HIT.strength(10.0) - 0.6).abs() < 1e-6);
        assert_eq!(HIT.strength(20.0), 1.0);
        assert_eq!(HIT.strength(80.0), 1.0, "no more than full");
        assert_eq!(HIT.strength(-5.0), 0.2, "a speed that opens is the floor");
        assert_eq!(HIT.strength(f32::NAN), 0.0);
    }

    #[test]
    fn a_scrape_is_nothing_below_the_start_and_full_from_the_full_speed() {
        assert_eq!(SCRAPE.strength(0.5), 0.0);
        assert_eq!(SCRAPE.strength(1.0), 0.0);
        assert!((SCRAPE.strength(2.0) - 0.5).abs() < 1e-6);
        assert_eq!(SCRAPE.strength(3.0), 1.0);
        assert_eq!(SCRAPE.strength(9.0), 1.0);
        assert_eq!(SCRAPE.strength(f32::INFINITY), 0.0);
    }

    #[test]
    fn a_channel_with_nothing_to_play_has_no_strength() {
        let empty: HitChannel<u8> = HitChannel { gate: Some(HIT), items: Vec::new() };
        assert_eq!(empty.strength(10.0), None);
        let full = HitChannel { gate: Some(HIT), items: vec![1u8] };
        assert!((full.strength(10.0).unwrap() - 0.6).abs() < 1e-6);
        let ungated: ScrapeChannel<u8> = ScrapeChannel { gate: None, items: vec![1] };
        assert_eq!(ungated.strength(2.0), None);
        assert_eq!(ScrapeChannel { gate: Some(SCRAPE), items: vec![1u8] }.strength(2.0), Some(0.5));
        assert_eq!(HitChannel::<u8>::default().strength(1.0), None);
    }

    #[test]
    fn gates_are_needed_only_when_the_channel_plays_something() {
        assert_eq!(hit_gate(None, None, false), Ok(None));
        assert!(matches!(hit_gate(None, None, true), Err(ContentError::Field { field: "hit.closing_speed_full", .. })));
        assert!(matches!(
            hit_gate(Some(5.0), None, true),
            Err(ContentError::Field { field: "hit.intensity_floor", .. })
        ));
        assert!(matches!(hit_gate(Some(0.0), Some(0.1), true), Err(ContentError::Field { .. })), "full speed above 0");
        assert!(matches!(hit_gate(Some(5.0), Some(1.5), true), Err(ContentError::Field { .. })));
        assert!(hit_gate(Some(5.0), Some(0.0), false).unwrap().is_some(), "a gate that is given is checked and kept");
        assert_eq!(scrape_gate(None, None, false), Ok(None));
        assert!(scrape_gate(Some(2.0), Some(2.0), true).is_err(), "the start is below the full speed");
        assert!(scrape_gate(Some(-1.0), Some(2.0), true).is_err());
        assert_eq!(
            scrape_gate(Some(0.0), Some(2.0), true).unwrap(),
            Some(ScrapeGate { speed_start: 0.0, speed_full: 2.0 })
        );
    }
}
