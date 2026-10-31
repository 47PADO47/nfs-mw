//! Bikes that mods add (`kind = "bike"` in the car manifest, docs/modding/bikes.md): the two-wheel data of each bike, by
//! folder, the stats the shop and the lists read for it, and where its two wheels sit on the model.
//!
//! A bike is a mod car like any other: it has a car table entry, a parts database entry and a [`CarStats`] (the shop's
//! lists, the ratings and the physics builder read stats). The stats of a bike are not written by the mod: they are
//! derived from its two-wheel data by [`derived_stats`], so the lists show the bike's mass, engine and gearbox. Its
//! physics are the two-wheel dynamics (`blackbox_vehicle::bike`) built from the data itself.

use std::sync::{Arc, PoisonError, RwLock};

use blackbox_vehicle::bike::BikeSpec;
use blackbox_vehicle::stats::CarStats;

use super::wheel_layout::WheelLayout;
use crate::car::{BikeMount, WheelSetup};

/// The bikes of this run, by folder. Each publication leaks the new list, as the other per-car tables do.
static PUBLISHED: RwLock<&'static [(String, Arc<BikeSpec>)]> = RwLock::new(&[]);

/// Register the two-wheel data of the mod car in `folder`. A folder already registered keeps its first.
pub fn publish(folder: &str, spec: BikeSpec) {
    let mut published = PUBLISHED.write().unwrap_or_else(PoisonError::into_inner);
    if published.iter().any(|(known, _)| known.eq_ignore_ascii_case(folder)) {
        return;
    }
    let mut all: Vec<(String, Arc<BikeSpec>)> = published.to_vec();
    all.push((folder.to_owned(), Arc::new(spec)));
    *published = Box::leak(all.into_boxed_slice());
}

/// The two-wheel data of the mod car in `folder`. `None` for a stock car and for a mod car that is not a bike.
pub fn of(folder: &str) -> Option<Arc<BikeSpec>> {
    let published = *PUBLISHED.read().unwrap_or_else(PoisonError::into_inner);
    published.iter().find(|(known, _)| known.eq_ignore_ascii_case(folder)).map(|(_, spec)| Arc::clone(spec))
}

/// Whether the car in `folder` is a bike.
pub fn is_bike(folder: &str) -> bool {
    of(folder).is_some()
}

/// The stats the lists read for a bike, derived from its data: mass with the rider, weight balance, wheelbase, tyre sizes,
/// the engine's torque curve and limits, the gearbox, the tyres' grip limits and the brake torque at each wheel. The keys
/// a bike has no counterpart for (the differentials, nitrous, a turbo, the drift assist) take the generic car's value, and
/// no top speed is stated. The bike's motion is not built from these (it comes from the data); the four-wheel body that
/// stands in for the bike in the game's wall and object contacts is, and so are the shop's lists.
pub fn derived_stats(spec: &BikeSpec) -> CarStats {
    let generic = CarStats::default();
    let chassis = &spec.chassis;
    let brakes = &spec.brakes;
    let peak = spec.engine.peak_torque.max(f32::EPSILON);
    CarStats {
        mass_kg: spec.total_mass(),
        front_weight_pct: chassis.front_weight * 100.0,
        cg_height_m: chassis.cg_height,
        wheelbase_m: chassis.wheelbase,
        // A bike has no track; the width of the body box stands in for it.
        track_m: [chassis.half_dimensions.x * 2.0; 2],
        tire_radius_m: [spec.front.radius, spec.rear.radius],
        tire_width_m: [spec.front.width, spec.rear.width],
        idle_rpm: spec.engine.idle_rpm,
        redline_rpm: spec.engine.max_rpm,
        torque_curve: spec.engine.torque_curve.iter().map(|&(rpm, share)| [rpm, share * peak]).collect(),
        engine_braking: (spec.engine.friction_torque / peak).clamp(0.0, 1.0),
        gear_ratios: spec.transmission.gears.clone(),
        final_drive: spec.transmission.final_ratio,
        gear_efficiency: spec.transmission.efficiency,
        tire_grip: [spec.front.lateral_grip_limit, spec.rear.lateral_grip_limit],
        brake_torque_nm: [brakes.front_force * spec.front.radius, brakes.rear_force * spec.rear.radius],
        travel_m: spec.front_suspension.max_travel,
        ..generic
    }
}

/// Where the body origin of the bike is, forward of the model's origin (m): midway between the two hubs. The model is
/// centred on its wheelbase when the car gives no layout.
pub fn origin_x(layout: Option<&WheelLayout>) -> f32 {
    layout.map_or(0.0, |layout| (layout.front.x + layout.rear.x) * 0.5)
}

/// The wheels of a bike: the hubs of `layout` when the car gives one (`[wheels]`), else plus and minus half the wheelbase
/// and as high as the tyre radius. Both wheels are on the centre line.
pub fn wheel_setup(spec: &BikeSpec, layout: Option<&WheelLayout>) -> WheelSetup {
    let half = spec.chassis.wheelbase * 0.5;
    let (front, rear, widths) = match layout {
        Some(layout) => (
            [layout.front.x, 0.0, layout.front.z, layout.front.radius],
            [layout.rear.x, 0.0, layout.rear.z, layout.rear.radius],
            [layout.front.width, layout.rear.width],
        ),
        None => (
            [half, 0.0, spec.front.radius, spec.front.radius],
            [-half, 0.0, spec.rear.radius, spec.rear.radius],
            [spec.front.width, spec.rear.width],
        ),
    };
    WheelSetup {
        tire_offsets: [front, front, rear, rear],
        fe_compressions: [0.0; 2],
        skid_width: [widths[0], widths[0], widths[1], widths[1]],
        skid_width_kit_scale: Vec::new(),
        kit_wheel_offset: [Vec::new(), Vec::new()],
        camber: [0.0; 2],
        spoke_count: 0,
        bike: Some(BikeMount { rake: spec.chassis.rake, trail: spec.chassis.trail }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::car::mods::wheel_layout::Tyre;

    fn layout() -> WheelLayout {
        WheelLayout {
            front: Tyre { x: 0.7355, z: 0.324, face: 0.0, radius: 0.324, width: 0.12 },
            rear: Tyre { x: -0.739, z: 0.324, face: 0.0, radius: 0.324, width: 0.15 },
        }
    }

    #[test]
    fn a_published_bike_is_found_by_folder_in_any_case() {
        publish("BIKEREGISTRY1", BikeSpec::example());
        assert!(is_bike("bikeregistry1"));
        assert!(!is_bike("NOBIKE"));
        assert_eq!(of("BikeRegistry1").unwrap().chassis.mass, BikeSpec::example().chassis.mass);
        // A second publication keeps the first.
        let mut other = BikeSpec::example();
        other.chassis.mass = 1.0;
        publish("BIKEREGISTRY1", other);
        assert_eq!(of("BIKEREGISTRY1").unwrap().chassis.mass, BikeSpec::example().chassis.mass);
    }

    #[test]
    fn the_derived_stats_carry_the_bikes_own_numbers() {
        let spec = BikeSpec::example();
        let stats = derived_stats(&spec);
        assert_eq!(stats.mass_kg, spec.total_mass());
        assert_eq!(stats.wheelbase_m, spec.chassis.wheelbase);
        assert_eq!(stats.tire_radius_m, [spec.front.radius, spec.rear.radius]);
        assert_eq!(stats.gear_ratios, spec.transmission.gears);
        assert_eq!(stats.redline_rpm, spec.engine.max_rpm);
        // The curve is the peak times the share: the peak of the example is its stated peak torque.
        assert!((stats.peak_torque_nm() - spec.engine.peak_torque).abs() < 1e-3);
        assert!((stats.front_weight_pct - 50.0).abs() < 1e-4);
        let front = spec.brakes.front_force * spec.front.radius;
        let rear = spec.brakes.rear_force * spec.rear.radius;
        assert!((stats.brake_torque_nm[0] - front).abs() < 1e-3);
        assert!((stats.brake_torque_nm[1] - rear).abs() < 1e-3);
        assert!(stats.top_speed.is_none());
    }

    #[test]
    fn without_a_layout_the_hubs_are_a_wheelbase_apart_and_one_radius_high() {
        let spec = BikeSpec::example();
        let setup = wheel_setup(&spec, None);
        let half = spec.chassis.wheelbase / 2.0;
        assert_eq!(setup.tire_offsets[0], [half, 0.0, spec.front.radius, spec.front.radius]);
        assert_eq!(setup.tire_offsets[2], [-half, 0.0, spec.rear.radius, spec.rear.radius]);
        assert_eq!(setup.tire_offsets[1], setup.tire_offsets[0]);
        assert_eq!(setup.tire_offsets[3], setup.tire_offsets[2]);
        assert_eq!(setup.skid_width, [spec.front.width, spec.front.width, spec.rear.width, spec.rear.width]);
        assert_eq!(setup.bike, Some(BikeMount { rake: spec.chassis.rake, trail: spec.chassis.trail }));
        assert_eq!(origin_x(None), 0.0);
    }

    #[test]
    fn a_layout_puts_the_two_hubs_where_the_model_has_them() {
        let setup = wheel_setup(&BikeSpec::example(), Some(&layout()));
        assert_eq!(setup.tire_offsets[0], [0.7355, 0.0, 0.324, 0.324]);
        assert_eq!(setup.tire_offsets[2], [-0.739, 0.0, 0.324, 0.324]);
        assert_eq!(setup.skid_width, [0.12, 0.12, 0.15, 0.15]);
        assert!((origin_x(Some(&layout())) - (-0.00175)).abs() < 1e-6);
    }
}
