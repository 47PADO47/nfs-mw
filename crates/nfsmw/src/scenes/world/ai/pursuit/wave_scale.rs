//! The F8 "cops per wave" enhancement **[ours]**: the chase takes the heat table's cop counts scaled, so more cops
//! chase, more must be evaded before the backup, and more search during the cool-down. A change applies to a running
//! chase at once. The reinforcement intervals shrink by the same scale, so a larger wave also arrives in the time the
//! reference one takes. The reference counts and intervals stay the default; the active-car limit still caps the cars
//! on the road. A reinforcement of a scaled wave tries more spots in a step ([`spawn_tries`]).

use nfsmw_data::pursuit::{HELICOPTER, HeatRow};

use super::Pursuit;
use crate::modding::cop_rows::without_models;

/// The shortest scaled reinforcement interval, seconds: the original's fast-spawn alternation.
const SHORTEST_DELAY: f32 = 0.2;

/// The most spots of each kind a reinforcement of a scaled wave tries in one step.
const MOST_SPAWN_TRIES: u32 = 8;

/// How many spots of each kind a reinforcement tries in one step: the seek walk's, each one 40 m (the spawn spacing)
/// further along the road, and the fallback disc's. One at the reference scale, as the original does, and one more for each
/// further multiple up to [`MOST_SPAWN_TRIES`]. With one of each, the 40 m spacing from the cop placed last and the
/// fallback disc's points off the cop roads held a scaled wave to about one new cop a second.
pub(super) fn spawn_tries(scale: f32) -> u32 {
    match scale > 1.0 {
        true => (scale.ceil() as u32).clamp(1, MOST_SPAWN_TRIES),
        false => 1,
    }
}

/// The chase's reference rows and the scale its working rows were built with.
pub(in crate::scenes::world::ai) struct Wave {
    base: Vec<HeatRow>,
    scale: f32,
}

impl Wave {
    pub(super) fn of(rows: &[HeatRow]) -> Self {
        Self { base: rows.to_vec(), scale: 1.0 }
    }

    /// A reinforcement interval of `secs` shortened by the scale, never below [`SHORTEST_DELAY`] (or `secs`, when it
    /// is shorter already).
    pub(super) fn delay(&self, secs: f32) -> f32 {
        match self.scale > 1.0 {
            true => (secs / self.scale).max(SHORTEST_DELAY.min(secs)),
            false => secs,
        }
    }

    /// Drops the car `name` from the reference rows too, so a later rescale does not bring it back.
    pub(super) fn drop_model(&mut self, name: &str) {
        let base = std::mem::take(&mut self.base);
        self.base = without_models(base, &[name.to_owned()]);
    }
}

impl Pursuit {
    /// Builds the working rows from the reference rows at `scale`. A running chase's cops still to evade grow or shrink
    /// with it, keeping those already evaded.
    pub(in crate::scenes::world::ai) fn set_wave_scale(&mut self, scale: f32) {
        if scale == self.wave.scale {
            return;
        }
        self.budget.scale_required(scale / self.wave.scale);
        self.wave.scale = scale;
        self.rows = scaled(self.wave.base.clone(), scale);
    }

    /// New reference rows (the pursuit context changed): the working rows take them at the chase's scale.
    pub(in crate::scenes::world::ai) fn set_base_rows(&mut self, rows: Vec<HeatRow>) {
        self.rows = scaled(rows.clone(), self.wave.scale);
        self.wave.base = rows;
    }
}

/// `rows` with every count grown by `scale`, rounded, a non-zero count never below one. The helicopter stays one.
pub(super) fn scaled(mut rows: Vec<HeatRow>, scale: f32) -> Vec<HeatRow> {
    if scale <= 1.0 {
        return rows;
    }
    let grow = |n: u32| match n {
        0 => 0,
        n => ((n as f32 * scale).round() as u32).max(1),
    };
    for row in &mut rows {
        for cop in row.cops.iter_mut().filter(|cop| cop.name != HELICOPTER) {
            cop.count = grow(cop.count);
        }
        row.full_engagement_cops = grow(row.full_engagement_cops);
        row.patrol_cars = grow(row.patrol_cars);
    }
    rows
}

#[cfg(test)]
mod tests {
    use nfsmw_data::pursuit::CopRecord;

    use super::*;

    fn row() -> HeatRow {
        HeatRow {
            cops: vec![
                CopRecord { name: "copsporthench".into(), count: 10, chance: 50 },
                CopRecord { name: HELICOPTER.into(), count: 1, chance: 60 },
            ],
            patrol_cars: 1,
            full_engagement_cops: 25,
            time_between_first_four_spawns: 10.0,
            time_between_cop_spawns: 2.0,
        }
    }

    fn pursuit(scale: f32) -> Pursuit {
        let player = blackbox_vehicle::performance::Performance {
            top_speed: 60.0,
            acceleration: [2.0; blackbox_vehicle::performance::TABLE_SIZE],
            start_grip: 1.0,
            end_grip: 1.0,
        };
        let mut pursuit = Pursuit::new(1, vec![row()], player);
        pursuit.set_wave_scale(scale);
        pursuit
    }

    #[test]
    fn a_running_chase_takes_a_new_scale_from_its_reference_rows() {
        let mut chase = pursuit(1.0);
        assert_eq!(chase.budget.allowed(), 25);
        chase.budget.note_evaded(5);
        chase.set_wave_scale(4.0);
        assert_eq!((chase.rows[0].cops[0].count, chase.rows[0].full_engagement_cops), (40, 100));
        assert_eq!(chase.budget.allowed(), 95, "the cops to evade grow with the wave, the evaded ones stay");
        chase.set_wave_scale(1.0);
        assert_eq!(chase.rows, vec![row()], "back to the reference rows, not a rounded copy");
        chase.set_base_rows(vec![HeatRow { patrol_cars: 3, ..row() }]);
        chase.set_wave_scale(2.0);
        assert_eq!(chase.rows[0].patrol_cars, 6, "a new context's rows are scaled too");
    }

    #[test]
    fn the_reinforcements_arrive_faster_by_the_scale() {
        assert_eq!(pursuit(1.0).wave.delay(2.0), 2.0, "the reference keeps its intervals");
        assert_eq!(pursuit(4.0).wave.delay(10.0), 2.5);
        assert_eq!(pursuit(20.0).wave.delay(2.0), 0.2, "no faster than the fast-spawn alternation");
        assert_eq!(pursuit(20.0).wave.delay(0.1), 0.1);
    }

    #[test]
    fn a_scaled_wave_tries_more_spawn_spots() {
        assert_eq!(spawn_tries(1.0), 1, "the reference tries one of each, as the original");
        assert_eq!(spawn_tries(2.5), 3);
        assert_eq!(spawn_tries(20.0), MOST_SPAWN_TRIES);
    }

    #[test]
    fn the_reference_scale_keeps_the_rows() {
        assert_eq!(scaled(vec![row()], 1.0), vec![row()]);
    }

    #[test]
    fn a_larger_scale_grows_the_cars_the_budget_and_the_search_but_not_the_helicopter() {
        let [got] = scaled(vec![row()], 1.5).try_into().unwrap();
        assert_eq!((got.cops[0].count, got.cops[1].count), (15, 1));
        assert_eq!((got.full_engagement_cops, got.patrol_cars), (38, 2));
        assert_eq!((got.cops[0].chance, got.time_between_cop_spawns), (50, 2.0), "weights and timing stay");
    }
}
