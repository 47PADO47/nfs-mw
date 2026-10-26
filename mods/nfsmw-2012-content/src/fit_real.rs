//! The wheels of every 2012 car against the model's hubs, with the real install (ignored: needs `MW2012_GAME_DIR`).
//!
//! For each car the wheel setup the game draws is built twice, the old way (from the stats alone) and from the car's
//! `[wheels]` table, and each wheel's centre, outer face, inner face, height and radius are compared with the model's own
//! wheel mesh. Run with `--nocapture` for the table; the cars whose wheels were off by more than 2 cm are marked.

use blackbox_modding::content::parse_car;
use chameleon_bundle::Bundle;
use chameleon_vehicle::{Vehicle, read_stats};
use nfsmw_data::car::WheelSetup;
use nfsmw_data::car::mods::CarStats;
use nfsmw_data::car::mods::wheel_layout::{Tyre, WheelLayout};

use crate::fit::to_car_space;
use crate::source::Source;
use crate::stats::ImportedStats;
use crate::wheels;

/// Largest error in metres the wheels may have.
const TOLERANCE: f32 = 0.02;

/// The extent of wheel `wheel` of `vehicle` along an engine axis.
fn extent(vehicle: &Vehicle, wheel: usize, axis: usize) -> (f32, f32) {
    let lod = &vehicle.wheels[wheel].lods[0];
    lod.iter()
        .flat_map(|mesh| mesh.vertices.iter().map(move |v| v.position[axis]))
        .fold((f32::MAX, f32::MIN), |(low, high), v| (low.min(v), high.max(v)))
}

/// The error in metres of wheel `wheel` (placed as `setup`'s corner `corner`) against the model: the largest of the
/// centre's x, height, the outer and inner faces and the radius.
fn error(
    vehicle: &Vehicle,
    wheel: usize,
    mesh: usize,
    axle_x: f32,
    mid: f32,
    setup: &WheelSetup,
    corner: usize,
) -> f32 {
    let [x, y, z, radius] = setup.tire_offsets[corner];
    let hub = vehicle.wheels[wheel].position;
    // A right wheel is the mirror of the left one (the DB5's meshes are not mirrored in the file).
    let (l0, l1) = extent(vehicle, mesh, 0);
    let lateral = [hub[0].abs() + l0, hub[0].abs() + l1];
    let (outer, inner) = (lateral[0].max(lateral[1]), lateral[0].min(lateral[1]));
    let (v0, v1) = extent(vehicle, mesh, 1);
    let model_radius = (v1 - v0) / 2.0;
    let width = setup.skid_width[corner];
    let parts = [x - (axle_x - mid), z - hub[1], y.abs() - outer, y.abs() - width - inner, radius - model_radius];
    if std::env::var_os("WHEELS_DEBUG").is_some() && parts.iter().any(|p| p.abs() > TOLERANCE) {
        println!(
            "    wheel {wheel} corner {corner}: x {:.3} z {:.3} outer {:.3} inner {:.3} radius {:.3}",
            parts[0], parts[1], parts[2], parts[3], parts[4]
        );
    }
    parts.into_iter().map(f32::abs).fold(0.0, f32::max)
}

fn layout_of(table: &blackbox_modding::content::WheelLayout) -> WheelLayout {
    let tyre = |t: &blackbox_modding::content::Tyre| Tyre {
        x: t.x_m,
        z: t.z_m,
        face: t.face_m,
        radius: t.radius_m,
        width: t.width_m,
    };
    WheelLayout { front: tyre(&table.front), rear: tyre(&table.rear) }
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn every_car_has_its_wheels_on_the_hubs_of_its_model() {
    let Some(root) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let source = Source::open(std::path::Path::new(&root)).unwrap();
    let (mut old_off, mut new_off, mut cars) = (0, Vec::new(), 0);
    for file in source.cars() {
        let name = source.object_name(file).unwrap_or_default();
        let Ok(car) = source.load(file) else { continue };
        let bytes = source.bundle_bytes(file).unwrap();
        let mut raw = read_stats(&Bundle::parse(&bytes).unwrap()).unwrap();
        let old_stats = CarStats::from(&parse_car("X", &ImportedStats::of(&raw).manifest_table()).unwrap().stats);
        let (rear_end, front_end) = car.vehicle.body[0]
            .iter()
            .flat_map(|mesh| mesh.vertices.iter().map(|v| v.position[2]))
            .fold((f32::MAX, f32::MIN), |(low, high), z| (low.min(z), high.max(z)));
        let Some(placed) = wheels::place(&car.vehicle, (rear_end, front_end), raw.axle_z) else {
            println!("{} {name}: NO LAYOUT", file.number);
            new_off.push(name);
            continue;
        };
        cars += 1;
        raw.axle_z = placed.axles;
        let text = ImportedStats::of(&raw).manifest_table() + &placed.manifest_table();
        let manifest = parse_car("X", &text).unwrap_or_else(|e| {
            panic!(
                "{name}: {e}
{text}"
            )
        });
        let stats = CarStats::from(&manifest.stats);
        let layout = layout_of(&manifest.wheels.expect("the table is written"));
        let new = WheelSetup::from_layout(&stats, Some(&layout));
        let old = WheelSetup::from_stats(&old_stats);
        // The corners are front left, front right, rear right, rear left; the wheels by place.
        let mut order: Vec<usize> = (0..4).collect();
        order.sort_by(|&a, &b| {
            to_car_space(car.vehicle.wheels[b].position)[0].total_cmp(&to_car_space(car.vehicle.wheels[a].position)[0])
        });
        let mid = placed.layout.x_mid;
        let old_mid = car.vehicle.wheels.iter().map(|w| to_car_space(w.position)[0]).sum::<f32>() / 4.0;
        let (mut worst_old, mut worst_new) = (0.0f32, 0.0f32);
        for (pair, axle) in [(&order[..2], placed.axles[0]), (&order[2..], placed.axles[1])] {
            let mesh = *pair.iter().find(|&&w| car.vehicle.wheels[w].position[0] > 0.0).unwrap();
            for &wheel in pair {
                let left = car.vehicle.wheels[wheel].position[0] > 0.0;
                let front = axle == placed.axles[0];
                let corner = match (front, left) {
                    (true, true) => 0,
                    (true, false) => 1,
                    (false, false) => 2,
                    (false, true) => 3,
                };
                // The old fit centred the body on the mean of the four hubs, so the old wheels are compared on that.
                worst_old = worst_old.max(error(&car.vehicle, wheel, mesh, axle, old_mid, &old, corner));
                worst_new = worst_new.max(error(&car.vehicle, wheel, mesh, axle, mid, &new, corner));
            }
        }
        let replaced = (placed.axles[0] - raw_axle_hub(&car.vehicle, true)).abs() > 0.02
            || (placed.axles[1] - raw_axle_hub(&car.vehicle, false)).abs() > 0.02;
        let mark = |e: f32| if e > TOLERANCE { "OFF" } else { "ok " };
        println!(
            "{:>8} {name:<58} old {:5.1} cm {} | new {:5.1} cm {}{}",
            file.number,
            worst_old * 100.0,
            mark(worst_old),
            worst_new * 100.0,
            mark(worst_new),
            if replaced { "  (axle from the vehicle info)" } else { "" }
        );
        old_off += usize::from(worst_old > TOLERANCE);
        if worst_new > TOLERANCE {
            new_off.push(name);
        }
    }
    println!("{cars} cars: {old_off} had wheels off by more than 2 cm; now off: {new_off:?}");
    assert!(new_off.is_empty(), "{new_off:?}");
}

/// The mean x of the hubs of the front or the rear pair in the graphics spec.
fn raw_axle_hub(vehicle: &Vehicle, front: bool) -> f32 {
    let mut xs: Vec<f32> = vehicle.wheels.iter().map(|w| to_car_space(w.position)[0]).collect();
    xs.sort_by(f32::total_cmp);
    if front { (xs[2] + xs[3]) / 2.0 } else { (xs[0] + xs[1]) / 2.0 }
}
