//! Where a 2012 car's wheels are, carried into the car's manifest (`[wheels]`) so the game draws them on the model's hubs.
//!
//! The hubs come from the car's graphics spec. One car (the Venom GT Spyder) has front hubs that lie behind its doors,
//! which no car can have; for an axle whose hubs are not under the body's own front or rear, the axle the vehicle info
//! gives is used instead. Everything else (height, track, tyre size) is measured from the hub and the wheel's mesh.

use chameleon_vehicle::Vehicle;

use crate::fit::{Layout, measure_on, to_car_space};

/// The most a hub may lie behind the nose, or ahead of the tail, as a share of the body's length. The real cars reach 0.24.
const MAX_OVERHANG: f32 = 0.3;

/// One axle's left tyre in 2012 car space (x forward, before the car is centred on its wheelbase).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tyre {
    pub x: f32,
    pub z: f32,
    /// Distance from the centre line to the tyre's outer face.
    pub face: f32,
    pub radius: f32,
    pub width: f32,
}

/// The wheels of a car as the model has them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub layout: Layout,
    pub front: Tyre,
    pub rear: Tyre,
    /// The x of the front and rear axle the layout uses.
    pub axles: [f32; 2],
}

fn extent(vehicle: &Vehicle, wheel: usize, axis: usize) -> Option<(f32, f32)> {
    let lod = vehicle.wheels.get(wheel)?.lods.first()?;
    let (low, high) = lod
        .iter()
        .flat_map(|mesh| mesh.vertices.iter().map(move |v| v.position[axis]))
        .fold((f32::MAX, f32::MIN), |(low, high), v| (low.min(v), high.max(v)));
    (high > low).then_some((low, high))
}

fn tyre(vehicle: &Vehicle, wheel: usize, axle_x: f32) -> Option<Tyre> {
    let hub = vehicle.wheels[wheel].position;
    let (lateral_in, lateral_out) = extent(vehicle, wheel, 0)?;
    let (low, high) = extent(vehicle, wheel, 1)?;
    Some(Tyre {
        x: axle_x,
        z: hub[1],
        face: hub[0].abs() + lateral_out,
        radius: (high - low) / 2.0,
        width: lateral_out - lateral_in,
    })
}

/// Whether an axle at `x` can be under a body that runs from `rear_end` to `front_end` (`front` for the front axle).
fn plausible(x: f32, front: bool, rear_end: f32, front_end: f32) -> bool {
    let overhang = if front { front_end - x } else { x - rear_end };
    (0.0..=MAX_OVERHANG * (front_end - rear_end)).contains(&overhang)
}

/// Place the wheels of `vehicle`: `body` is the extent of the body along the car (rear end, front end) and `info_axles`
/// the x of the front and rear axle the vehicle info gives.
pub fn place(vehicle: &Vehicle, body: (f32, f32), info_axles: [f32; 2]) -> Option<Placed> {
    if vehicle.wheels.len() != 4 {
        return None;
    }
    // The wheels by position, never by name: a few cars have their wheels named the wrong way round.
    let mut order: Vec<usize> = (0..4).collect();
    order.sort_by(|&a, &b| {
        to_car_space(vehicle.wheels[b].position)[0].total_cmp(&to_car_space(vehicle.wheels[a].position)[0])
    });
    let hub_x = |wheel: usize| to_car_space(vehicle.wheels[wheel].position)[0];
    let hubs = [(hub_x(order[0]) + hub_x(order[1])) / 2.0, (hub_x(order[2]) + hub_x(order[3])) / 2.0];
    let mut axles = hubs;
    for (end, axle) in axles.iter_mut().enumerate() {
        if plausible(*axle, end == 0, body.0, body.1) {
            continue;
        }
        if !plausible(info_axles[end], end == 0, body.0, body.1) {
            return None;
        }
        log::warn!(
            "the hubs of the {} axle lie outside the body: the vehicle info's axle is used",
            ["front", "rear"][end]
        );
        *axle = info_axles[end];
    }
    let left = |pair: [usize; 2]| pair.into_iter().find(|&wheel| vehicle.wheels[wheel].position[0] > 0.0);
    let (front_wheel, rear_wheel) = (left([order[0], order[1]])?, left([order[2], order[3]])?);
    Some(Placed {
        layout: measure_on(vehicle, Some(axles))?,
        front: tyre(vehicle, front_wheel, axles[0])?,
        rear: tyre(vehicle, rear_wheel, axles[1])?,
        axles,
    })
}

impl Placed {
    /// The `[wheels]` tables of the manifest: the tyres as the model has them once the car is centred on its wheelbase.
    pub fn manifest_table(&self) -> String {
        let mid = self.layout.x_mid;
        let mut text = String::new();
        for (name, tyre) in [("front", &self.front), ("rear", &self.rear)] {
            text.push_str(&format!("\n[wheels.{name}]\n"));
            for (key, value) in [
                ("x_m", tyre.x - mid),
                ("z_m", tyre.z),
                ("face_m", tyre.face),
                ("radius_m", tyre.radius),
                ("width_m", tyre.width),
            ] {
                let rounded = (value * 10_000.0).round() / 10_000.0;
                text.push_str(&format!("{key} = {rounded:?}\n"));
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_axle_under_the_body_is_plausible_and_one_behind_the_doors_is_not() {
        assert!(plausible(1.3, true, -2.33, 2.33));
        assert!(plausible(-1.5, false, -2.33, 2.33));
        assert!(!plausible(0.125, true, -2.33, 2.33));
        assert!(!plausible(3.0, true, -2.33, 2.33), "past the nose");
    }

    #[test]
    fn the_table_is_in_the_centred_space() {
        let tyre = Tyre { x: 1.3, z: 0.33, face: 0.93, radius: 0.32, width: 0.26 };
        let layout = Layout { wheelbase: 2.8, x_mid: -0.1, track: 0.9, hub_z: 0.33, radius: 0.32 };
        let rear = Tyre { x: -1.5, ..tyre };
        let text = Placed { layout, front: tyre, rear, axles: [1.3, -1.5] }.manifest_table();
        assert!(text.contains("[wheels.front]\nx_m = 1.4\n"), "{text}");
        assert!(text.contains("[wheels.rear]\nx_m = -1.4\n"), "{text}");
    }
}
