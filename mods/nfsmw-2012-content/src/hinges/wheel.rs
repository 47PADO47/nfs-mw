//! The steering wheel as a hinged part that the steering turns (`[damage_parts.steering]`,
//! `docs/modding/car-damage-parts.md`).
//!
//! The wheel is the part of the body skinned to the `SteeringWheel` bone. The conversion cuts it out of the finest body solid
//! as `<ID>_KIT00_STEERING_WHEEL_A` and `_B`, like the other hinged parts, and the manifest tells the game to turn it about the
//! column by the steering input times the lock angle. Spec: `docs/formats/chameleon-driver-animation.md`.

use chameleon_vehicle::{DEFAULT_LOCK_DEGREES, HingedPart, STEERING_BONE, Skeleton, Vehicle, WheelFrame, wheel_frame};

use super::{Fit, Part, to_car_space};

/// The zone a wheel is listed under (the front; the damage never turns or removes it).
const ZONE: u16 = 1;
/// What the wheel weighs, kg: only the manifest's range needs it, the wheel never leaves the car.
const MASS_KG: f32 = 2.0;
/// Fewest vertices a wheel is read from.
const MIN_VERTICES: usize = 24;

/// How the steering turns a part: about the part's pivot along `axis` (in the 2012 space, pointing forward) by the steering
/// input times `lock_degrees`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wheel {
    pub axis: [f32; 3],
    /// The degrees at full steering, each way.
    pub lock_degrees: f32,
}

/// The hinged part of the `SteeringWheel` bone, added to `parts` unless a part owns that bone already. Returns the bone, or
/// `None` when the car has none.
pub fn add_part(parts: &mut Vec<HingedPart>, skeleton: &Skeleton) -> Option<usize> {
    let bone = skeleton.find(STEERING_BONE)?;
    if parts.iter().any(|part| part.bone == bone) {
        return None;
    }
    parts.push(HingedPart {
        name: STEERING_BONE.to_owned(),
        bone,
        zone: ZONE,
        centre: skeleton.bones[bone].position,
        limits: [[0.0; 2]; 3],
        mass_kg: MASS_KG,
        thresholds: Vec::new(),
        targets: Vec::new(),
    });
    Some(bone)
}

/// The pivot and the turn of the wheel: the part `part` of `classes` (the classification of the finest body level's
/// triangles) in the body `vehicle` holds, with the lock angle the car's data gives. `None` when the part owns too little
/// geometry or it is not a flat disc.
pub fn frame(
    vehicle: &Vehicle,
    classes: &[Vec<Option<usize>>],
    part: usize,
    lock: Option<f32>,
) -> Option<(WheelFrame, Wheel)> {
    let body = vehicle.body.first()?;
    let mut points = Vec::new();
    for (mesh, classes) in body.iter().zip(classes) {
        let mut taken = vec![false; mesh.vertices.len()];
        for (triangle, class) in mesh.triangles.iter().zip(classes) {
            if *class != Some(part) {
                continue;
            }
            for &index in triangle {
                let Some(taken) = taken.get_mut(index as usize) else { continue };
                if std::mem::replace(taken, true) {
                    continue;
                }
                points.push(mesh.vertices[index as usize].position);
            }
        }
    }
    if points.len() < MIN_VERTICES {
        return None;
    }
    let frame = wheel_frame(&points)?;
    let wheel = Wheel { axis: frame.axis, lock_degrees: lock.unwrap_or(DEFAULT_LOCK_DEGREES) };
    Some((frame, wheel))
}

/// The `[[damage_parts]]` table of the wheel part `part` whose solid is `stem`: it never turns or leaves with damage, and the
/// `[damage_parts.steering]` table turns it with the steering. The pivot and axis are moved into the car's space by `fit`.
pub fn table(part: &Part, wheel: &Wheel, stem: &str, fit: &Fit) -> String {
    let pivot = fit.apply(to_car_space(part.pivot));
    let axis = to_car_space(wheel.axis);
    format!(
        "\n[[damage_parts]]\nname = \"{}\"\nmodel = \"{stem}\"\nzones = [\"front\"]\n\
         pivot = [{:.5}, {:.5}, {:.5}]\naxis = [{:.5}, {:.5}, {:.5}]\nangle = 0.0\ndetach = false\nmass_kg = {MASS_KG:.1}\n\
         \n[damage_parts.steering]\nlock_degrees = {:.1}\n",
        part.name, pivot[0], pivot[1], pivot[2], axis[0], axis[1], axis[2], wheel.lock_degrees
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fit::Layout;

    fn part(wheel: bool) -> Part {
        Part {
            name: STEERING_BONE.into(),
            zone: ZONE,
            pivot: [0.36, 0.9, 0.2],
            limits: [[0.0; 2]; 3],
            thresholds: Vec::new(),
            mass_kg: MASS_KG,
            active: None,
            parent: None,
            delay: None,
            drives: Vec::new(),
            tracks: Vec::new(),
            damage: None,
            steering: wheel.then_some(Wheel { axis: [0.0, 0.3, 0.95], lock_degrees: 400.0 }),
        }
    }

    #[test]
    fn the_table_turns_the_wheel_about_its_column_in_the_cars_axes_and_keeps_it_on_the_car() {
        let layout = Layout { wheelbase: 2.6, x_mid: 0.0, track: 0.8, hub_z: 0.35, radius: 0.35 };
        let wheel = Wheel { axis: [0.0, 0.3, 0.95], lock_degrees: 400.0 };
        let text = table(&part(true), &wheel, "CAR_KIT00_STEERING_WHEEL", &Fit::own(layout));
        let manifest = blackbox_modding::content::parse_car("CAR", &text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        let entry = &manifest.damage_parts[0];
        assert_eq!((entry.name.as_str(), entry.model.as_str()), ("SteeringWheel", "CAR_KIT00_STEERING_WHEEL"));
        assert_eq!(entry.steering, Some(blackbox_modding::content::SteeredPart { lock_degrees: 400.0 }));
        assert!(!entry.detach && entry.angle == 0.0, "the damage neither turns nor removes the wheel");
        // The engine's forward (z) is the game's x; up (y) is z.
        assert!(
            entry.axis[0] > 0.9 && (entry.axis[2] - 0.3).abs() < 0.01 && entry.axis[1].abs() < 1e-6,
            "{:?}",
            entry.axis
        );
    }

    #[test]
    fn a_car_without_the_bone_has_no_wheel_part() {
        let mut parts = Vec::new();
        let skeleton = Skeleton { bones: Vec::new() };
        assert_eq!(add_part(&mut parts, &skeleton), None);
        assert!(parts.is_empty());
    }

    /// With the install: every car with a `SteeringWheel` bone gets a wheel part that owns triangles of the finest body level,
    /// whose pivot lies within 30 cm of the bone, whose column points forward (away from the driver) and lies at a right angle
    /// to the line between the driver's two grips, with a lock angle from the car's data.
    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
    fn every_car_has_a_wheel_of_triangles_turning_about_its_column() {
        use chameleon_bundle::Bundle;
        use chameleon_vehicle::{driver_grips, driver_setup};
        let Some(root) = std::env::var_os("MW2012_GAME_DIR") else { return };
        let source = crate::source::Source::open(std::path::Path::new(&root)).unwrap();
        let (mut cars, mut worst_off, mut worst_tilt, mut worst_grip) = (0, 0.0f32, 1.0f32, 0.0f32);
        let (mut locks, mut missing) = (std::collections::BTreeSet::new(), Vec::new());
        for file in source.cars() {
            let bytes = source.bundle_bytes(file).unwrap();
            let bundle = Bundle::parse(&bytes).unwrap();
            let Some(skeleton) = Skeleton::read(&bundle) else { continue };
            let Some(bone) = skeleton.find(STEERING_BONE) else { continue };
            let car = source.load(file).unwrap();
            let Some(hinged) = car.hinged else {
                missing.push(file.number);
                continue;
            };
            let Some(at) = hinged.parts.iter().position(|part| part.name == STEERING_BONE) else {
                missing.push(file.number);
                continue;
            };
            let part = &hinged.parts[at];
            let wheel = part.steering.expect("the wheel turns with the steering");
            let owned = hinged.classes.iter().flatten().filter(|class| **class == Some(at)).count();
            assert!(owned > 20, "{}: the wheel owns {owned} triangles", file.number);
            let position = skeleton.bones[bone].position;
            let off = (0..3).map(|k| (part.pivot[k] - position[k]).powi(2)).sum::<f32>().sqrt();
            assert!(off < 0.3, "{}: the pivot is {off} m from the bone", file.number);
            worst_off = worst_off.max(off);
            assert!(wheel.axis[2] > 0.6, "{}: the column {:?} does not point forward", file.number, wheel.axis);
            worst_tilt = worst_tilt.min(wheel.axis[2]);
            locks.insert(wheel.lock_degrees as i32);
            // The tables the converter writes for the car are accepted by the game, with the wheel among them.
            let stems: Vec<String> =
                hinged.parts.iter().map(|part| format!("CAR_KIT00_{}", crate::hinges::stem(&part.name))).collect();
            let layout = Layout { wheelbase: 2.6, x_mid: 0.0, track: 0.8, hub_z: 0.35, radius: 0.35 };
            let text = crate::hinges::manifest_tables(&hinged, &stems, &Fit::own(layout));
            let manifest = blackbox_modding::content::parse_car("CAR", &text).unwrap_or_else(|e| {
                panic!(
                    "{e}
{text}"
                )
            });
            let entry = manifest.damage_parts.iter().find(|part| part.model == "CAR_KIT00_STEERING_WHEEL");
            let entry = entry.unwrap_or_else(|| panic!("{}: the manifest has no wheel", file.number));
            assert_eq!(entry.steering.map(|s| s.lock_degrees), Some(wheel.lock_degrees));
            let grips = driver_setup(&bundle)
                .and_then(|setup| setup.seat.and_then(|seat| driver_grips(&bundle, &setup, seat).map(|g| (g, seat))));
            if let Some((grips, _)) = grips {
                let between: [f32; 3] = std::array::from_fn(|k| grips.left[k] - grips.right[k]);
                let length = between.iter().map(|c| c * c).sum::<f32>().sqrt();
                let cosine = (0..3).map(|k| between[k] * wheel.axis[k]).sum::<f32>().abs() / length;
                assert!(cosine < 0.45, "{}: the grips are not in the wheel's plane (cos {cosine})", file.number);
                worst_grip = worst_grip.max(cosine);
            }
            cars += 1;
        }
        println!("no wheel: {missing:?}");
        println!(
            "{cars} wheels, lock angles {locks:?}, pivot {worst_off:.3} m from the bone, least forward {worst_tilt:.2}, grips cos {worst_grip:.2}"
        );
        // Five cars keep their wheel in the body: on three no triangle is skinned to the bone, one has a bone that owns
        // more than a wheel (not a flat disc), and one has no usable skin.
        assert!(cars >= 85 && missing.len() <= 6, "{cars} cars have a wheel, none on {missing:?}");
    }
}
