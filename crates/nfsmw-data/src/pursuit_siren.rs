//! The siren timings of the pursuit tables: six fields of each heat level's `pursuitlevels` row, and the two
//! `speechtune` values that decide whether a chase is "high intensity" (which picks the harsher siren states).
//! Spec: `docs/specs/cop-siren-audio.md` section 2.

use blackbox_attrib::Database;

use crate::pursuit::{HEAT_LEVELS, level_collection};

/// The siren times of one heat level, seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SirenRow {
    pub init_min_period: f32,
    pub init_variation: f32,
    pub wail_period: f32,
    pub max_yelp_time: f32,
    pub scream_period: f32,
    pub max_scream_time: f32,
}

impl Default for SirenRow {
    /// The values every heat row of the install has, used when the database lacks a row.
    fn default() -> Self {
        Self {
            init_min_period: 0.5,
            init_variation: 1.5,
            wail_period: 11.0,
            max_yelp_time: 6.0,
            scream_period: 60.0,
            max_scream_time: 3.0,
        }
    }
}

/// The siren data of the install: the row of each heat level and the intensity marks.
#[derive(Debug, Clone, PartialEq)]
pub struct SirenData {
    /// The row of heat level `n` at index `n - 1`.
    pub rows: Vec<SirenRow>,
    /// `HighIntensityMark`: the heat at or above which a chase is high intensity.
    pub high_intensity_heat: u32,
    /// `PursuitDurationHighIntensity`: the chase length (seconds) at or above which it is high intensity.
    pub high_intensity_secs: f32,
}

impl Default for SirenData {
    fn default() -> Self {
        Self { rows: Vec::new(), high_intensity_heat: 2, high_intensity_secs: 150.0 }
    }
}

impl SirenData {
    /// The row of heat level `heat` (clamped to 1…10); the default row when the table lacks the level.
    pub fn row(&self, heat: u32) -> SirenRow {
        let index = heat.clamp(1, HEAT_LEVELS) as usize - 1;
        self.rows.get(index).copied().unwrap_or_default()
    }

    /// Whether a chase at `heat` that has run `secs` seconds is high intensity.
    pub fn is_high_intensity(&self, heat: u32, secs: f32) -> bool {
        heat >= self.high_intensity_heat || secs >= self.high_intensity_secs
    }
}

/// The siren data of the install.
pub fn siren_data(db: &Database) -> SirenData {
    let defaults = SirenRow::default();
    let rows = (1..=HEAT_LEVELS)
        .map(|heat| {
            let Some(row) = level_collection(db, "heattable", heat) else { return defaults };
            let time = |field: &str, fallback: f32| row.get_f32(field).unwrap_or(fallback);
            SirenRow {
                init_min_period: time("SirenInitMinPeriod", defaults.init_min_period),
                init_variation: time("SirenInitVariation", defaults.init_variation),
                wail_period: time("SirenWailPeriod", defaults.wail_period),
                max_yelp_time: time("SirenMaxYelpTime", defaults.max_yelp_time),
                scream_period: time("SirenScreamPeriod", defaults.scream_period),
                max_scream_time: time("SirenMaxScreamTime", defaults.max_scream_time),
            }
        })
        .collect();
    let mut data = SirenData { rows, ..SirenData::default() };
    if let Some(tune) = db.collections_of("speechtune").next() {
        data.high_intensity_heat = tune.get_i32("HighIntensityMark").map_or(2, |v| v.max(0) as u32);
        data.high_intensity_secs = tune.get_f32("PursuitDurationHighIntensity").unwrap_or(150.0);
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_level_gives_the_default_row() {
        assert_eq!(SirenData::default().row(3), SirenRow::default());
        assert_eq!(SirenData::default().row(0), SirenRow::default());
    }

    #[test]
    fn the_rows_are_by_level_and_clamped() {
        let slow = SirenRow { wail_period: 20.0, ..SirenRow::default() };
        let data = SirenData { rows: vec![SirenRow::default(), slow], ..SirenData::default() };
        assert_eq!(data.row(2), slow);
        assert_eq!(data.row(9), SirenRow::default(), "past the table");
    }

    #[test]
    fn a_chase_is_intense_by_heat_or_by_length() {
        let data = SirenData::default();
        assert!(!data.is_high_intensity(1, 149.0));
        assert!(data.is_high_intensity(2, 0.0));
        assert!(data.is_high_intensity(1, 150.0));
    }
}
