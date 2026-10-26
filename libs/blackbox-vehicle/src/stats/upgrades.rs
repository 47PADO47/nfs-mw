//! Shop upgrades of a car with its own stats: the same car after `levels` of each part, as a function of its own numbers.
//!
//! A level changes the stats of the car by a share of what the car already has, so a strong engine gains more newton
//! metres than a weak one and a car with grippy tyres gains more grip. The step sizes:
//!
//! | Category | Per level (3 levels at most) |
//! |---|---|
//! | Tires | tyre friction coefficient + 5 percent of the car's own, both axles |
//! | Brakes | brake torque + 10 percent |
//! | Suspension | spring frequency + 6 percent, damping ratio + 0.03, ride height - 8 mm, centre of gravity - 15 mm, downforce + 15 percent |
//! | Transmission | gear efficiency + 0.03 (at most 1) |
//! | Engine | torque + 6 percent at every engine speed |
//! | Induction | `induction_boost` + 0.08 |
//! | Nitrous | level 1 is the car's factory nitrous (a car without one gets a kit of 0.4 boost for 4 s), each further level + 15 percent boost and + 25 percent burn time |

use super::CarStats;
use crate::VehicleSpec;

/// Levels a category has at most.
pub const MAX_LEVEL: u8 = 3;

/// The shop's categories, in the order of the game's performance package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Tires,
    Brakes,
    Suspension,
    Transmission,
    Engine,
    Induction,
    Nitrous,
}

impl Category {
    pub const ALL: [Self; 7] =
        [Self::Tires, Self::Brakes, Self::Suspension, Self::Transmission, Self::Engine, Self::Induction, Self::Nitrous];
}

const TIRE_GRIP: f32 = 0.05;
const BRAKE_TORQUE: f32 = 0.10;
const SPRING_FREQUENCY: f32 = 0.06;
const DAMPING_RATIO: f32 = 0.03;
const RIDE_HEIGHT_M: f32 = 0.008;
const CG_HEIGHT_M: f32 = 0.015;
const DOWNFORCE: f32 = 0.15;
const GEAR_EFFICIENCY: f32 = 0.03;
const ENGINE_TORQUE: f32 = 0.06;
const INDUCTION_BOOST: f32 = 0.08;
const NITROUS_BOOST_STEP: f32 = 0.15;
const NITROUS_TIME_STEP: f32 = 0.25;
/// The kit a car without factory nitrous gets at level 1.
const KIT_BOOST: f32 = 0.4;
const KIT_SECONDS: f32 = 4.0;
/// Floors the suspension steps stop at.
const MIN_RIDE_HEIGHT_M: f32 = 0.03;
const MIN_CG_HEIGHT_M: f32 = 0.1;

/// The stats of `stats` with the upgrades of `levels`, indexed like [`Category::ALL`]. A level above [`MAX_LEVEL`] counts
/// as the maximum.
///
/// A car whose air drag is solved from a top speed keeps the drag it has at stock: an upgrade makes the car faster, it
/// does not move the target.
pub fn upgrade(stats: &CarStats, levels: [u8; 7]) -> CarStats {
    let mut car = stats.clone();
    if levels.iter().all(|&level| level == 0) {
        return car;
    }
    if stats.top_speed.is_some() {
        car.drag = VehicleSpec::from_stats(stats, glam::Vec3::ONE).aero.drag_coefficient;
        car.top_speed = None;
    }
    for category in Category::ALL {
        let level = levels[category as usize].min(MAX_LEVEL);
        if level > 0 {
            apply(&mut car, stats, category, f32::from(level));
        }
    }
    car
}

fn apply(car: &mut CarStats, own: &CarStats, category: Category, level: f32) {
    match category {
        Category::Tires => car.tire_grip = own.tire_grip.map(|grip| grip * (1.0 + TIRE_GRIP * level)),
        Category::Brakes => car.brake_torque_nm = own.brake_torque_nm.map(|nm| nm * (1.0 + BRAKE_TORQUE * level)),
        Category::Suspension => {
            car.ride_frequency_hz = own.ride_frequency_hz.map(|hz| hz * (1.0 + SPRING_FREQUENCY * level));
            car.damping_ratio = (own.damping_ratio + DAMPING_RATIO * level).min(1.0);
            car.ride_height_m =
                (own.ride_height_m - RIDE_HEIGHT_M * level).max(MIN_RIDE_HEIGHT_M.min(own.ride_height_m));
            car.cg_height_m = (own.cg_height_m - CG_HEIGHT_M * level).max(MIN_CG_HEIGHT_M.min(own.cg_height_m));
            car.downforce = own.downforce * (1.0 + DOWNFORCE * level);
        }
        Category::Transmission => car.gear_efficiency = (own.gear_efficiency + GEAR_EFFICIENCY * level).min(1.0),
        Category::Engine => {
            car.torque_curve =
                own.torque_curve.iter().map(|[rpm, nm]| [*rpm, nm * (1.0 + ENGINE_TORQUE * level)]).collect();
        }
        Category::Induction => car.induction_boost = own.induction_boost + INDUCTION_BOOST * level,
        Category::Nitrous => {
            let (boost, seconds) = match own.nitrous {
                true => (own.nitrous_boost, own.nitrous_seconds),
                false => (KIT_BOOST, KIT_SECONDS),
            };
            car.nitrous = true;
            car.nitrous_boost = boost * (1.0 + NITROUS_BOOST_STEP * (level - 1.0));
            car.nitrous_seconds = seconds * (1.0 + NITROUS_TIME_STEP * (level - 1.0));
        }
    }
}
