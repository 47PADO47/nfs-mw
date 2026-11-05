//! The F8 "cops per wave" enhancement **[ours]**: a chase that starts with it takes the heat table's cop counts scaled,
//! so more cops chase, more must be evaded before the backup, and more search during the cool-down. The reference
//! counts stay the default; the active-car limit still caps the cars on the road.

use nfsmw_data::pursuit::{HELICOPTER, HeatRow};

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
