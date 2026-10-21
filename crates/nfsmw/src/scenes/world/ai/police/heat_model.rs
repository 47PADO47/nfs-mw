//! How the career shapes a chase's heat: the heat range of the race bin and the bin's growth time.
//! Spec: `docs/specs/ai-pursuit-heat.md` sections 3.1 and 3.2.
//!
//! The race bin is the career rank bucket: 15 at the start of the Blacklist, 1 near its end. Without a career the bin is
//! 15, the rewrite's earlier bin-free times (600 s a level for levels 1 to 5) are the times of bins 15 and 14.

use nfsmw_data::pursuit::HeatGrowth;

use super::Police;
use crate::settings::bartender::BartenderSettings;

/// The heat range without a career or a config range: the whole scale (a free-roam chase may be forced to any level).
const FREE_ROAM: (f32, f32) = (1.0, 10.0);

/// The bin a world without a career is in: the start of the Blacklist.
const NO_CAREER_BIN: u8 = 15;
/// The last bin index the growth scale has (`min(current_bin, 14)`).
const LAST_SCALE_BIN: u8 = 14;

/// The heat range of a race bin, `(base, max)` (`gameplay.bin` `race_bin_NN`, `BaseOpenWorldHeat` and `MaxOpenWorldHeat`,
/// install-verified in the heat spec section 3.1). Bins 15 and 16 start at heat 1.
pub fn bin_heat_range(bin: u8) -> (f32, f32) {
    match bin {
        16 => (1.0, 1.0),
        15 => (1.0, 2.0),
        13..=14 => (0.0, 2.0),
        9..=12 => (0.0, 3.0),
        5..=8 => (0.0, 4.0),
        _ => (0.0, 5.0),
    }
}

/// The career's effect on heat for one world.
#[derive(Debug, Clone, Default)]
pub struct HeatModel {
    /// The Blacklist's current bin while a career is on.
    bin: Option<u8>,
    /// The growth of heat level `n` at index `n - 1`, from the install.
    growth: Vec<Option<HeatGrowth>>,
}

impl HeatModel {
    /// The install's growth tables.
    pub fn set_growth(&mut self, growth: Vec<Option<HeatGrowth>>) {
        self.growth = growth;
    }

    /// The Blacklist's current bin, or `None` when no career is on.
    pub fn set_bin(&mut self, bin: Option<u8>) {
        self.bin = bin;
    }

    /// The range a chase forced or started at `level` keeps its heat in. A config range wins, then the career bin's, then
    /// the whole scale; a chase above the range raises it.
    pub fn range_for(&self, config: Option<(f32, f32)>, level: u32) -> (f32, f32) {
        let (min, max) = config.or(self.bin.map(bin_heat_range)).unwrap_or(FREE_ROAM);
        (min, max.max(level as f32))
    }

    /// Seconds to climb one level at `level` for these settings: the Bartender file's value when it sets one, else the
    /// install's time for the bin.
    pub fn growth_seconds(&self, settings: &BartenderSettings, level: u32) -> f32 {
        let configured = settings.time_per_level(level);
        match settings.time_per_level_is_stock(level) {
            true => self.seconds_per_level(level, configured),
            false => configured,
        }
    }

    /// Seconds to climb one heat level at `level`: `TimePerHeatLevel` times the bin's `ScaleEscalationPerBucket` entry,
    /// or `fallback` when the install table lacks the level.
    pub fn seconds_per_level(&self, level: u32, fallback: f32) -> f32 {
        let Some(growth) = self.growth.get(level.max(1) as usize - 1).copied().flatten() else { return fallback };
        let bin = self.bin.unwrap_or(NO_CAREER_BIN);
        growth.seconds_for_bin(u32::from(bin.min(LAST_SCALE_BIN)))
    }
}

impl Police {
    /// The Blacklist's current bin, or `None` without a career: it sets the heat range and the growth time of a chase.
    pub fn set_career_bin(&mut self, bin: Option<u8>) {
        self.heat_model.set_bin(bin);
    }

    /// The install's heat growth tables (`nfsmw_data::pursuit::career_heat`).
    pub fn set_heat_growth(&mut self, growth: Vec<Option<HeatGrowth>>) {
        self.heat_model.set_growth(growth);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVEL_ONE: [f32; 15] =
        [120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 120.0, 180.0, 240.0, 360.0, 600.0];

    fn model(bin: Option<u8>) -> HeatModel {
        let mut model = HeatModel::default();
        model.set_growth(vec![Some(HeatGrowth { time_per_heat_level: 1.0, scale_per_bucket: LEVEL_ONE })]);
        model.set_bin(bin);
        model
    }

    #[test]
    fn the_heat_range_follows_the_race_bin() {
        assert_eq!(bin_heat_range(16), (1.0, 1.0));
        assert_eq!(bin_heat_range(15), (1.0, 2.0));
        assert_eq!(bin_heat_range(14), (0.0, 2.0));
        assert_eq!(bin_heat_range(13), (0.0, 2.0));
        assert_eq!(bin_heat_range(12), (0.0, 3.0));
        assert_eq!(bin_heat_range(9), (0.0, 3.0));
        assert_eq!(bin_heat_range(8), (0.0, 4.0));
        assert_eq!(bin_heat_range(5), (0.0, 4.0));
        assert_eq!(bin_heat_range(4), (0.0, 5.0));
        assert_eq!(bin_heat_range(1), (0.0, 5.0));
    }

    #[test]
    fn a_world_without_a_career_has_no_range_of_its_own() {
        assert_eq!(model(None).range_for(None, 1), (1.0, 10.0));
        assert_eq!(model(Some(11)).range_for(None, 1), (0.0, 3.0));
        assert_eq!(model(Some(11)).range_for(Some((2.0, 4.0)), 1), (2.0, 4.0), "a config range wins");
        assert_eq!(model(Some(15)).range_for(None, 6), (1.0, 6.0), "a forced chase raises the range");
    }

    #[test]
    fn a_level_one_climb_gets_quicker_down_the_blacklist() {
        assert_eq!(model(Some(15)).seconds_per_level(1, 600.0), 600.0);
        assert_eq!(model(Some(14)).seconds_per_level(1, 600.0), 600.0);
        assert_eq!(model(Some(13)).seconds_per_level(1, 600.0), 360.0);
        assert_eq!(model(Some(12)).seconds_per_level(1, 600.0), 240.0);
        assert_eq!(model(Some(11)).seconds_per_level(1, 600.0), 180.0);
        assert_eq!(model(Some(10)).seconds_per_level(1, 600.0), 120.0);
        assert_eq!(model(Some(1)).seconds_per_level(1, 600.0), 120.0);
    }

    #[test]
    fn without_a_career_the_start_of_the_blacklist_times_apply() {
        assert_eq!(model(None).seconds_per_level(1, 99.0), 600.0);
    }

    #[test]
    fn a_level_the_table_lacks_keeps_the_fallback() {
        assert_eq!(model(Some(1)).seconds_per_level(2, 777.0), 777.0);
        assert_eq!(HeatModel::default().seconds_per_level(1, 600.0), 600.0);
    }
}
