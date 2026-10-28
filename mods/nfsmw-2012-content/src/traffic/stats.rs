//! The `[stats]` table of a traffic car: the body and wheels of the car as its model has them, with the numbers of its class for
//! everything the traffic bundle does not say (`class.rs`). The game builds the physics of a mod car from this table and its
//! `BOUNDS.BIN`, and its traffic AI drives the car with them.

use crate::stats::ImportedStats;
use crate::wheels::Placed;

use super::class::{Drive, Facts, numbers};

/// Points of the torque curve.
const CURVE_SHARES: [(f32, f32); 5] = [(0.35, 0.72), (0.65, 0.93), (1.0, 1.0), (1.45, 0.92), (1.8, 0.74)];
/// Brake torque per wheel as a multiple of the car's mass, N m per kg, for the front and the rear axle.
const BRAKE_PER_KG: [f32; 2] = [0.9, 0.62];

/// The stats of the traffic car whose wheels are `placed` and whose facts are `facts`.
pub fn of(placed: &Placed, facts: &Facts) -> ImportedStats {
    let n = numbers(facts.class);
    let wheelbase = placed.layout.wheelbase;
    // The tyres' centres are half a tyre's width inside their outer face.
    let track = |tyre: &crate::wheels::Tyre| 2.0 * (tyre.face - tyre.width / 2.0);
    let (drive, front_share) = match facts.drive {
        Drive::Front => ("fwd", 1.0),
        Drive::Rear => ("rwd", 0.0),
    };
    let mut curve: Vec<[f32; 2]> = vec![[n.idle_rpm, n.peak_torque_nm * 0.62]];
    curve.extend(CURVE_SHARES.iter().map(|&(rpm, torque)| [n.peak_rpm * rpm, n.peak_torque_nm * torque]));
    // The curve ends at the redline: the last point cannot be above it, and the points before it keep their order.
    curve.retain(|point| point[0] < n.redline_rpm);
    curve.push([n.redline_rpm, n.peak_torque_nm * 0.6]);
    ImportedStats {
        mass_kg: n.mass_kg,
        front_weight_pct: n.front_weight_pct,
        cg_height_m: n.cg_height_m,
        wheelbase_m: wheelbase.clamp(1.0, 6.0),
        track_m: [track(&placed.front).clamp(0.8, 3.5), track(&placed.rear).clamp(0.8, 3.5)],
        tire_radius_m: [placed.front.radius.clamp(0.15, 0.8), placed.rear.radius.clamp(0.15, 0.8)],
        tire_width_m: [placed.front.width.clamp(0.1, 0.5), placed.rear.width.clamp(0.1, 0.5)],
        ride_height_m: n.ride_height_m,
        idle_rpm: n.idle_rpm,
        redline_rpm: n.redline_rpm,
        torque_curve: curve,
        gear_ratios: n.gears.to_vec(),
        reverse_ratio: n.reverse_ratio,
        final_drive: n.final_drive,
        drive,
        front_torque_share: front_share,
        diff_lock: [0.0, 0.0],
        top_speed_kph: n.top_speed_kph,
        tire_grip: [n.tire_grip, n.tire_grip],
        brake_torque_nm: [n.mass_kg * BRAKE_PER_KG[0], n.mass_kg * BRAKE_PER_KG[1]],
        nitrous_boost: 0.0,
        drift: None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::class::TrafficClass;
    use super::*;
    use crate::fit::Layout;
    use crate::wheels::Tyre;
    use blackbox_modding::content::parse_car;

    fn placed() -> Placed {
        let tyre = |x: f32| Tyre { x, z: 0.325, face: 0.9, radius: 0.325, width: 0.2 };
        Placed {
            layout: Layout { wheelbase: 2.58, x_mid: 0.34, track: 0.9, hub_z: 0.325, radius: 0.325 },
            front: tyre(1.63),
            rear: tyre(-0.95),
            axles: [1.63, -0.95],
        }
    }

    fn facts(class: TrafficClass, drive: Drive) -> Facts {
        Facts { class, drive, display: None }
    }

    #[test]
    fn the_wheels_are_the_models_and_the_rest_is_the_classs() {
        let stats = of(&placed(), &facts(TrafficClass::Compact, Drive::Front));
        assert_eq!(stats.wheelbase_m, 2.58);
        assert!((stats.track_m[0] - 1.6).abs() < 1e-5, "twice the distance from the middle to a tyre's centre");
        assert_eq!((stats.tire_radius_m[0], stats.tire_width_m[1]), (0.325, 0.2));
        assert_eq!((stats.drive, stats.front_torque_share), ("fwd", 1.0));
        assert_eq!(stats.mass_kg, 1300.0);
        assert_eq!(stats.nitrous_boost, 0.0, "a traffic car has no nitrous");
    }

    #[test]
    fn the_torque_curve_rises_to_the_peak_and_ends_at_the_redline() {
        for class in [TrafficClass::Compact, TrafficClass::Sedan, TrafficClass::Van, TrafficClass::Truck] {
            let stats = of(&placed(), &facts(class, Drive::Rear));
            let curve = &stats.torque_curve;
            assert!(curve.windows(2).all(|pair| pair[1][0] > pair[0][0]), "{class:?}: rpm rises: {curve:?}");
            assert_eq!(curve.last().unwrap()[0], stats.redline_rpm);
            assert!(stats.idle_rpm < curve.last().unwrap()[0]);
            assert!((2..=16).contains(&curve.len()));
            let peak = curve.iter().map(|point| point[1]).fold(0.0, f32::max);
            assert_eq!(peak, numbers(class).peak_torque_nm);
        }
    }

    #[test]
    fn the_table_is_one_the_games_parser_accepts() {
        for (class, drive) in [(TrafficClass::Compact, Drive::Front), (TrafficClass::Truck, Drive::Rear)] {
            let text = format!("role = \"traffic\"\n{}", of(&placed(), &facts(class, drive)).manifest_table());
            let manifest = parse_car("audi_tr", &text).unwrap_or_else(|error| panic!("{class:?}: {error}\n{text}"));
            assert_eq!(manifest.stats.mass_kg, numbers(class).mass_kg);
        }
    }
}
