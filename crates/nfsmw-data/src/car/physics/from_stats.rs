//! A car built from its own stats (`blackbox_vehicle::stats::CarStats`): the physics of a car that a mod adds.
//!
//! Nothing here reads a stock car. The spec is the vehicle library's `VehicleSpec::from_stats`; this file adds what the
//! game wraps around a spec: the collision box the caller measured, the wall contact, the jump configuration and the
//! shop's upgrade catalogue.

use blackbox_vehicle::VehicleSpec;
use blackbox_vehicle::stats::{CarStats, Category, MAX_LEVEL, upgrade};

use super::performance::{CategoryOptions, PerformanceCatalog, PerformanceCategory};
use super::{CarBounds, CarPhysics, PerformancePackage, WallSpec};
use crate::pursuit::AiVehicle;

/// The share of the downforce that stays while no wheel touches the ground. The racers of the game keep 80 percent.
const AIRBORNE_DOWNFORCE: f32 = 0.8;

impl CarPhysics {
    /// The car of `stats` with the collision box `bounds`.
    ///
    /// The body box (`bounds.half_dimensions`) gives the inertia and the ground contact. A car built here has no contact
    /// geometry, event sequencer, collision reactions or effect source of its own: those belong to the caller, which
    /// fills `geometry`, `events`, `reactions` and `effect_source` from the car's own bounds.
    pub fn from_stats(stats: &CarStats, bounds: CarBounds) -> Self {
        let mut spec = VehicleSpec::from_stats(stats, bounds.half_dimensions);
        spec.aero.airborne_scale = AIRBORNE_DOWNFORCE;
        Self {
            effect_source: 0,
            spec,
            tire_integration: blackbox_vehicle::tires::TireIntegration::Reference,
            jump_config: super::jump_config(),
            bounds,
            walls: WallSpec::default(),
            geometry: None,
            events: None,
            reactions: None,
        }
    }
}

/// The car of `stats` after the shop upgrades of `package`, with the collision box `bounds`. The junkman parts of the
/// package are ignored: a car with its own stats has none.
pub fn physics_from_stats(stats: &CarStats, package: PerformancePackage, bounds: CarBounds) -> CarPhysics {
    CarPhysics::from_stats(&upgrade(stats, package.levels), bounds)
}

impl PerformanceCatalog {
    /// What the shop offers a car with its own stats: every category up to [`MAX_LEVEL`], no junkman parts. The turbo and
    /// the nitrous count as installed when the stats give them.
    pub fn from_stats(stats: &CarStats) -> Self {
        let categories = PerformanceCategory::ALL.map(|category| CategoryOptions {
            max_level: MAX_LEVEL,
            missing_endpoints: false,
            junkman: false,
            stock_present: match category {
                PerformanceCategory::Induction => stats.induction_boost > 0.0,
                PerformanceCategory::Nitrous => stats.nitrous,
                _ => true,
            },
        });
        Self { categories }
    }
}

/// The levels of the vehicle library's categories, in the order of [`PerformanceCategory`]: the two lists must agree.
const _: () = {
    assert!(Category::Tires as usize == PerformanceCategory::Tires as usize);
    assert!(Category::Brakes as usize == PerformanceCategory::Brakes as usize);
    assert!(Category::Suspension as usize == PerformanceCategory::Chassis as usize);
    assert!(Category::Transmission as usize == PerformanceCategory::Transmission as usize);
    assert!(Category::Engine as usize == PerformanceCategory::Engine as usize);
    assert!(Category::Induction as usize == PerformanceCategory::Induction as usize);
    assert!(Category::Nitrous as usize == PerformanceCategory::Nitrous as usize);
};

/// How the AI drives a car with its own stats: it may use all of the car's top speed, and its acceleration is the car's.
/// A car without a stated top speed gets the AI's default limit.
pub fn ai_vehicle_from_stats(stats: &CarStats) -> AiVehicle {
    let default = AiVehicle::default();
    AiVehicle { max_speed_kmh: stats.top_speed.map_or(default.max_speed_kmh, |ms| ms * 3.6), ..default }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    const BOUNDS: CarBounds = CarBounds { half_dimensions: Vec3::new(0.91, 0.55, 2.21), pivot: Vec3::ZERO };

    fn stats() -> CarStats {
        CarStats {
            mass_kg: 1397.0,
            torque_curve: vec![[1000.0, 360.0], [5600.0, 440.0], [7800.0, 390.0]],
            idle_rpm: 1000.0,
            redline_rpm: 7800.0,
            gear_ratios: vec![3.91, 2.29, 1.55, 1.3, 1.08, 0.88, 0.71],
            final_drive: 3.44,
            top_speed: Some(84.5),
            ..CarStats::default()
        }
    }

    #[test]
    fn the_physics_use_the_callers_box_and_the_stats() {
        let physics = CarPhysics::from_stats(&stats(), BOUNDS);
        assert_eq!(physics.spec.dimension, BOUNDS.half_dimensions);
        assert_eq!(physics.bounds, BOUNDS);
        assert_eq!(physics.spec.mass, 1397.0);
        assert_eq!(physics.spec.aero.airborne_scale, AIRBORNE_DOWNFORCE);
        assert!(physics.geometry.is_none() && physics.events.is_none() && physics.reactions.is_none());
    }

    #[test]
    fn the_upgrades_of_a_package_change_the_car() {
        let stats = stats();
        let stock = physics_from_stats(&stats, PerformancePackage::default(), BOUNDS);
        let package = PerformancePackage { levels: [0, 0, 0, 0, MAX_LEVEL, 0, MAX_LEVEL], junkman: 0xFF };
        let tuned = physics_from_stats(&stats, package, BOUNDS);
        assert!(tuned.spec.engine.torque[8] > stock.spec.engine.torque[8]);
        assert!(tuned.spec.nos.present() && !stock.spec.nos.present());
        // The drag a top speed gave the stock car stays.
        assert_eq!(tuned.spec.aero.drag_coefficient, stock.spec.aero.drag_coefficient);
    }

    #[test]
    fn the_catalogue_offers_every_category_and_knows_what_is_installed() {
        let mut stats = stats();
        let catalog = PerformanceCatalog::from_stats(&stats);
        assert!(catalog.categories.iter().all(|option| option.max_level == MAX_LEVEL && !option.junkman));
        assert!(!catalog.categories[PerformanceCategory::Nitrous as usize].stock_present);
        stats.nitrous = true;
        assert!(PerformanceCatalog::from_stats(&stats).categories[PerformanceCategory::Nitrous as usize].stock_present);
        assert_eq!(catalog.maximum().levels, [MAX_LEVEL; 7]);
    }

    #[test]
    fn the_ai_may_use_the_whole_top_speed() {
        assert!((ai_vehicle_from_stats(&stats()).max_speed_kmh - 84.5 * 3.6).abs() < 1e-3);
        let unstated = CarStats { top_speed: None, ..stats() };
        assert_eq!(ai_vehicle_from_stats(&unstated), AiVehicle::default());
    }
}
