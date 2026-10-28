//! A 2012 car's drift numbers as `[stats]` keys (`drift_*`, docs/modding/car-stats.md): when a slide is a drift, the largest
//! angle its physics works to and the yaw torque that holds a drift. The game reads them only while the drift assist is on
//! (on only for the mod's own cars, `drift_nitrous.rs`), so a stock car is what it was without them. The numbers
//! are read by `chameleon_vehicle::read_drift_handling` (docs/formats/chameleon-drift.md).

use chameleon_vehicle::DriftHandling;

use crate::stats::number;

/// The ranges of the game's `drift_*` keys (libs/blackbox-modding/src/content/stats.rs); imported numbers are clamped to them.
const SPEED: (f32, f32) = (2.0, 60.0);
const ENTER_ANGLE: (f32, f32) = (2.0, 30.0);
const EXIT_ANGLE: (f32, f32) = (1.0, 20.0);
const MAX_ANGLE: (f32, f32) = (10.0, 70.0);
const TORQUE: (f32, f32) = (0.0, 200_000.0);
/// The exit angle stays this far under the enter angle, degrees.
const HYSTERESIS: f32 = 0.5;

/// A car's `drift_*` keys.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftKeys {
    pub enter_speed_ms: f32,
    pub enter_angle_deg: f32,
    pub exit_angle_deg: f32,
    /// 0 when the car's data names none (two cars): the key is left out and the game's generic angle applies.
    pub max_angle_deg: f32,
    /// 0 leaves the key out: the game's generic strength applies.
    pub yaw_torque_nm: f32,
}

impl DriftKeys {
    /// The keys of a car's drift handling.
    pub fn of(handling: &DriftHandling) -> Self {
        let physics = &handling.physics;
        let enter_angle_deg = physics.trigger_angle.clamp(ENTER_ANGLE.0, ENTER_ANGLE.1);
        Self {
            enter_speed_ms: physics.trigger_speed.clamp(SPEED.0, SPEED.1),
            enter_angle_deg,
            exit_angle_deg: physics.exit_angle.clamp(EXIT_ANGLE.0, EXIT_ANGLE.1).min(enter_angle_deg - HYSTERESIS),
            max_angle_deg: match handling.max_angle > 0.0 {
                true => handling.max_angle.clamp(MAX_ANGLE.0, MAX_ANGLE.1),
                false => 0.0,
            },
            yaw_torque_nm: handling.yaw_torque_nm.clamp(TORQUE.0, TORQUE.1),
        }
    }

    /// The lines of the keys, for the end of the `[stats]` table.
    pub fn lines(&self) -> String {
        let mut text = String::new();
        let mut line = |key: &str, value: f32| text.push_str(&format!("{key} = {}\n", number(value)));
        line("drift_enter_speed_ms", self.enter_speed_ms);
        line("drift_enter_angle_deg", self.enter_angle_deg);
        line("drift_exit_angle_deg", self.exit_angle_deg);
        if self.max_angle_deg > 0.0 {
            line("drift_max_angle_deg", self.max_angle_deg);
        }
        if self.yaw_torque_nm > 0.0 {
            line("drift_yaw_torque_nm", self.yaw_torque_nm);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use blackbox_modding::content::parse_car;
    use chameleon_vehicle::DriftPhysics;

    use super::*;

    fn handling() -> DriftHandling {
        DriftHandling {
            physics: DriftPhysics { trigger_speed: 10.0, trigger_angle: 8.5, exit_angle: 3.0, exit_hold: 0.25 },
            max_angle: 45.0,
            yaw_torque_nm: 37_500.0,
            yaw_torque_steps: [12_500.0, 7_750.0, 1_500.0],
            handbrake_torque_nm: 1_400.0,
            handbrake_second_nm: 650.0,
        }
    }

    #[test]
    fn a_cars_numbers_become_its_keys() {
        let keys = DriftKeys::of(&handling());
        assert_eq!(
            keys.lines(),
            "drift_enter_speed_ms = 10.0\ndrift_enter_angle_deg = 8.5\ndrift_exit_angle_deg = 3.0\ndrift_max_angle_deg = 45.0\ndrift_yaw_torque_nm = 37500.0\n"
        );
    }

    #[test]
    fn a_car_without_an_angle_or_a_torque_leaves_the_key_out() {
        let bare = DriftHandling { max_angle: 0.0, yaw_torque_nm: 0.0, ..handling() };
        let text = DriftKeys::of(&bare).lines();
        assert!(!text.contains("drift_max_angle_deg") && !text.contains("drift_yaw_torque_nm"), "{text}");
    }

    #[test]
    fn numbers_outside_the_games_ranges_are_clamped_and_the_exit_stays_under_the_enter_angle() {
        let wild = DriftHandling {
            physics: DriftPhysics { trigger_speed: 100.0, trigger_angle: 2.2, exit_angle: 5.0, exit_hold: 0.25 },
            yaw_torque_nm: 1.0e9,
            ..handling()
        };
        let keys = DriftKeys::of(&wild);
        assert_eq!((keys.enter_speed_ms, keys.yaw_torque_nm), (60.0, 200_000.0));
        assert!(keys.exit_angle_deg < keys.enter_angle_deg);
    }

    #[test]
    fn the_lines_are_a_stats_table_the_game_accepts() {
        let text = format!("[stats]\n{}", DriftKeys::of(&handling()).lines());
        let manifest = match parse_car("test", &text) {
            Err(_) => parse_car("test", &format!("clone = \"X\"{}{text}", '\n')).unwrap(),
            Ok(manifest) => manifest,
        };
        assert_eq!(manifest.stats.drift.enter_angle_deg, 8.5);
        assert_eq!(manifest.stats.drift.yaw_torque_nm, 37_500.0);
    }
}
