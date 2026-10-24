//! How the career shapes a chase's heat: the heat range of the race bin and the bin's growth time.
//! Spec: `docs/specs/ai-pursuit-heat.md` sections 3.1 and 3.2.
//!
//! The race bin is the career rank bucket: 15 at the start of the Blacklist, 1 near its end. Without a career profile,
//! the raw growth query uses the selected row's base time. The ordinary sandbox adapter uses the initial career
//! growth bucket instead. Event context is specified in `docs/specs/ai-pursuit-context.md`.

use nfsmw_data::pursuit::{EventHeat, HeatGrowth, PursuitTable};

use super::Police;
use crate::settings::bartender::BartenderSettings;

/// Remake sandbox policy: ordinary free roam uses the normal five levels without needing a career profile.
const FREE_ROAM: (f32, f32) = (1.0, 5.0);

/// The last bin index the growth scale has (`min(current_bin, 14)`).
const LAST_SCALE_BIN: u8 = 14;

/// World/event rules kept independently of an active pursuit.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PursuitContext {
    pub racing: bool,
    /// Actual racing play mode; an event's loading, intro and results retain world heat tables.
    pub race_heat_active: bool,
    pub pursuit_event: bool,
    pub career_event: bool,
    pub challenge: bool,
    pub final_pursuit: bool,
    pub event_heat: EventHeat,
}

impl PursuitContext {
    pub fn table(self) -> PursuitTable {
        match self.racing && self.race_heat_active && !self.pursuit_event {
            true => PursuitTable::Race,
            false => PursuitTable::OpenWorld,
        }
    }
}

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
    context: PursuitContext,
    /// Explicit development starts can exceed ordinary event/career limits.
    forced: bool,
    pursuit_options: crate::settings::PursuitOptions,
}

impl HeatModel {
    pub(super) fn cop_limit(&self, reference: u32) -> u32 {
        self.pursuit_options.active_limit(reference)
    }

    /// The install's growth tables.
    pub fn set_growth(&mut self, growth: Vec<Option<HeatGrowth>>) {
        self.growth = growth;
    }

    /// The Blacklist's current bin, or `None` when no career is on.
    pub fn set_bin(&mut self, bin: Option<u8>) {
        self.bin = bin;
    }

    /// The range a chase forced or started at `level` keeps its heat in. A config range wins, then the career bin's, then
    /// ordinary free roam's five levels; an explicitly forced chase above the range raises it.
    pub fn range_for(&self, config: Option<(f32, f32)>, level: u32) -> (f32, f32) {
        if self.context.final_pursuit {
            return (6.0, 6.0);
        }
        let ordinary = match self.context {
            PursuitContext { racing: true, career_event: true, .. } => {
                self.bin.map(bin_heat_range).unwrap_or((1.0, 10.0))
            }
            PursuitContext { racing: true, challenge: false, .. } => (1.0, 5.0),
            PursuitContext { racing: true, .. } => self.context.event_heat.range(),
            _ => self.bin.map(bin_heat_range).unwrap_or(FREE_ROAM),
        };
        let (mut min, mut max) = config.unwrap_or(ordinary);
        let override_max = self.pursuit_options.maximum_heat();
        if let Some(override_max) = override_max {
            min = min.min(override_max);
            max = override_max;
        }
        let max = match self.forced && override_max.is_none() {
            true => max.max(level as f32),
            false => max,
        };
        (min.clamp(0.0, 10.0), max.clamp(min.clamp(0.0, 10.0), 10.0))
    }

    /// Seconds to climb one level at `level` for these settings: the Bartender file's value when it sets one, else the
    /// install's time for the bin. Ordinary no-profile free roam uses the initial career growth bucket as a remake
    /// sandbox policy; `seconds_per_level` keeps the original raw profile-absent query.
    pub fn growth_seconds(&self, settings: &BartenderSettings, level: u32) -> f32 {
        let configured = settings.time_per_level(level);
        if !settings.time_per_level_is_stock(level) {
            return configured;
        }
        if self.bin.is_none() && !self.context.racing && !self.context.challenge && !self.context.final_pursuit {
            let Some(growth) = self.growth.get(level.max(1) as usize - 1).copied().flatten() else {
                return configured;
            };
            return growth.seconds_for_bin(u32::from(LAST_SCALE_BIN));
        }
        self.seconds_per_level(level, configured)
    }

    /// Seconds to climb one heat level at `level`: `TimePerHeatLevel` times the bin's `ScaleEscalationPerBucket` entry,
    /// or `fallback` when the install table lacks the level.
    pub fn seconds_per_level(&self, level: u32, fallback: f32) -> f32 {
        let Some(growth) = self.growth.get(level.max(1) as usize - 1).copied().flatten() else { return fallback };
        let Some(mut bin) = self.bin else { return growth.time_per_heat_level };
        if self.context.challenge {
            bin = LAST_SCALE_BIN;
        }
        growth.seconds_for_bin(u32::from(bin.min(LAST_SCALE_BIN)))
    }
}

impl Police {
    pub fn set_pursuit_options(&mut self, options: crate::settings::PursuitOptions) {
        let options = options.sanitized();
        if self.heat_model.pursuit_options == options {
            return;
        }
        self.heat_model.pursuit_options = options;
        self.settings.max_cops = options.active_limit(self.configured_max_cops);
        self.reclamp_heat();
    }

    pub fn set_pursuit_context(&mut self, context: PursuitContext) {
        self.heat_model.context = context;
        self.heat_model.forced = false;
        self.reclamp_heat();
    }

    fn reclamp_heat(&mut self) {
        if let Some(old) = self.heat {
            let config = crate::settings::extra_options::active().heat_bounds().map(|bounds| (bounds.min, bounds.max));
            let (min, max) = self.heat_model.range_for(config, old.level());
            let mut heat = blackbox_pursuit::Heat::new(min, max);
            heat.begin();
            heat.raise_to(old.value());
            self.heat = Some(heat);
        }
    }

    pub fn pursuit_context(&self) -> PursuitContext {
        self.heat_model.context
    }

    pub fn final_pursuit(&self) -> bool {
        self.heat_model.context.final_pursuit
    }

    pub fn set_forced_pursuit(&mut self, forced: bool) {
        self.heat_model.forced = forced;
    }
    /// The Blacklist's current bin, or `None` without a career: it sets the heat range and the growth time of a chase.
    pub fn set_career_bin(&mut self, bin: Option<u8>) {
        self.heat_model.set_bin(bin);
    }

    /// The install's heat growth tables (`nfsmw_data::pursuit::career_heat`).
    pub fn set_heat_growth(&mut self, growth: Vec<Option<HeatGrowth>>) {
        self.heat_model.set_growth(growth);
    }

    pub fn heat_growth_seconds(&self, level: u32) -> f32 {
        self.heat_model.growth_seconds(&self.settings, level)
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
    fn a_world_without_a_career_uses_the_ordinary_sandbox_range() {
        assert_eq!(model(None).range_for(None, 1), (1.0, 5.0));
        assert_eq!(model(Some(11)).range_for(None, 1), (0.0, 3.0));
        assert_eq!(model(Some(11)).range_for(Some((2.0, 4.0)), 1), (2.0, 4.0), "a config range wins");
        let mut forced = model(Some(15));
        forced.forced = true;
        assert_eq!(forced.range_for(None, 6), (1.0, 6.0), "a forced chase raises the range");
        assert_eq!(model(Some(15)).range_for(None, 6), (1.0, 2.0), "an automatic chase obeys the bin cap");
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
    fn without_a_career_no_bucket_multiplier_applies() {
        assert_eq!(model(None).seconds_per_level(1, 99.0), 1.0);
    }

    #[test]
    fn sandbox_growth_uses_the_initial_bucket_without_changing_raw_or_event_queries() {
        let settings = BartenderSettings::default();
        let mut model = model(None);
        assert_eq!(model.seconds_per_level(1, 99.0), 1.0);
        assert_eq!(model.growth_seconds(&settings, 1), 600.0);
        model.set_bin(Some(10));
        assert_eq!(model.growth_seconds(&settings, 1), 120.0);
        model.set_bin(None);
        model.context.racing = true;
        assert_eq!(model.growth_seconds(&settings, 1), 1.0);
        model.context.career_event = true;
        assert_eq!(model.range_for(None, 1), (1.0, 10.0), "profile-absent career-event rules stay unchanged");
        model.context.career_event = false;
        model.context.challenge = true;
        assert_eq!(model.growth_seconds(&settings, 1), 1.0, "profile-absent challenge arithmetic stays raw");
        model.set_bin(Some(1));
        assert_eq!(model.growth_seconds(&settings, 1), 600.0, "profiled challenges keep bucket 14");
        model.set_bin(None);
        model.context = PursuitContext { final_pursuit: true, ..Default::default() };
        assert_eq!(model.growth_seconds(&settings, 1), 1.0);
        assert_eq!(model.range_for(None, 1), (6.0, 6.0));
    }

    #[test]
    fn nonstock_growth_overrides_the_sandbox_bucket_and_career_bucket() {
        let mut settings = BartenderSettings::default();
        settings.levels[0].time_per_level = 90.0;
        assert_eq!(model(None).growth_seconds(&settings, 1), 90.0);
        assert_eq!(model(Some(10)).growth_seconds(&settings, 1), 90.0);
    }

    #[test]
    fn sandbox_special_rows_require_an_explicit_start_or_cap() {
        let mut model = model(None);
        assert_eq!(model.range_for(None, 9), (1.0, 5.0));
        model.forced = true;
        assert_eq!(model.range_for(None, 9), (1.0, 9.0));
        model.forced = false;
        model.pursuit_options = crate::settings::PursuitOptions { enhanced: true, heat_cap: 8, ..Default::default() };
        assert_eq!(model.range_for(None, 1), (1.0, 8.0));
    }

    #[test]
    fn a_level_the_table_lacks_keeps_the_fallback() {
        assert_eq!(model(Some(1)).seconds_per_level(2, 777.0), 777.0);
        assert_eq!(HeatModel::default().seconds_per_level(1, 600.0), 600.0);
    }

    #[test]
    fn quick_races_have_a_five_heat_cap_even_with_a_loaded_career() {
        let mut model = model(Some(15));
        model.context = PursuitContext { racing: true, ..Default::default() };
        assert_eq!(model.range_for(None, 8), (1.0, 5.0));
        model.context.career_event = true;
        assert_eq!(model.range_for(None, 8), (1.0, 2.0));
    }

    #[test]
    fn challenges_use_their_bin_range_and_the_last_growth_bucket() {
        let mut model = model(Some(1));
        model.context = PursuitContext {
            racing: true,
            challenge: true,
            event_heat: EventHeat { bin_range: (0.0, 7.0), ..Default::default() },
            ..Default::default()
        };
        assert_eq!(model.range_for(None, 7), (0.0, 7.0));
        assert_eq!(model.seconds_per_level(1, 99.0), 600.0);
        model.context.event_heat.bin_range = (0.0, 10.0);
        assert_eq!(model.range_for(None, 9), (0.0, 10.0), "special event rows are available without guessed mappings");
    }

    #[test]
    fn the_epic_pursuit_has_fixed_heat_even_with_mod_and_development_overrides() {
        let mut model = model(Some(1));
        model.context.final_pursuit = true;
        model.forced = true;
        assert_eq!(model.range_for(Some((1.0, 10.0)), 10), (6.0, 6.0));
    }

    #[test]
    fn pursuit_events_keep_the_open_world_table_while_other_races_switch() {
        let mut context = PursuitContext { racing: true, race_heat_active: true, ..Default::default() };
        assert_eq!(context.table(), PursuitTable::Race);
        context.pursuit_event = true;
        assert_eq!(context.table(), PursuitTable::OpenWorld);
        context.racing = false;
        assert_eq!(context.table(), PursuitTable::OpenWorld);
    }

    #[test]
    fn optional_limits_can_be_enabled_and_removed_during_a_chase() {
        let mut police = Police::default();
        police.set_career_bin(Some(15));
        police.begin(1);
        let options = crate::settings::PursuitOptions { enhanced: true, max_active_cops: 24, heat_cap: 7 };
        police.set_pursuit_options(options);
        assert_eq!(police.settings().max_cops, 24);
        assert_eq!(police.heat_model.range_for(None, 1), (1.0, 7.0));
        police.set_carried_heat(6.0);
        police.begin(6);
        assert_eq!(police.level(), Some(6));
        police.set_pursuit_options(Default::default());
        assert_eq!(police.settings().max_cops, 8);
        assert_eq!(police.level(), Some(2), "restoring reference clamps the active chase back to career bounds");
        police.heat_model.context.final_pursuit = true;
        police.set_pursuit_options(options);
        assert_eq!(police.heat_model.range_for(None, 1), (6.0, 6.0));
    }

    #[test]
    fn active_car_options_keep_a_development_heat_but_explicit_heat_caps_win() {
        let mut police = Police::default();
        police.set_career_bin(Some(15));
        police.set_forced_pursuit(true);
        police.begin(6);
        let options = crate::settings::PursuitOptions { enhanced: true, max_active_cops: 24, heat_cap: 0 };
        police.set_pursuit_options(options);
        assert_eq!(police.level(), Some(6), "enabling a car limit must preserve the forced heat");
        police.set_pursuit_options(crate::settings::PursuitOptions { max_active_cops: 32, ..options });
        assert_eq!(police.level(), Some(6), "changing the car limit must preserve the forced heat");
        police.set_pursuit_options(Default::default());
        assert_eq!(police.level(), Some(6), "disabling only the car override also keeps the development heat");
        police.set_pursuit_options(crate::settings::PursuitOptions { heat_cap: 4, ..options });
        assert_eq!(police.level(), Some(4), "a selected heat cap overrides a development start");
        police.begin(9);
        assert_eq!(police.level(), Some(4), "new development starts also obey an explicit heat cap");
        police.set_pursuit_options(Default::default());
        police.set_pursuit_context(PursuitContext::default());
        assert_eq!(police.level(), Some(2), "event context changes clear the development range");
        police.heat_model.context.final_pursuit = true;
        police.set_pursuit_options(crate::settings::PursuitOptions { heat_cap: 4, ..options });
        assert_eq!(police.level(), Some(6), "the authored final heat wins over the explicit cap");
    }
}
