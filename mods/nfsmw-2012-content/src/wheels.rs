//! Where a 2012 car's wheels are, carried into the car's manifest (`[wheels]`) so the game draws them on the model's hubs.
//!
//! The hubs come from the car's graphics spec. For an axle whose hubs are not under the body's own front or rear (the
//! Venom GT Spyder's front hubs lie behind its doors), or where the body has no arch for the tyre but has one at the axle
//! the vehicle info gives (the BMW 1M, whose body is another car's; the E46 GTR's rear), that axle is used instead.
//! Everything else (height, track, tyre size) is measured from the hub and the wheel's mesh.

use chameleon_vehicle::{TyreVolume, Vehicle, prefer_other};

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

/// The tyre of `wheel` as the arch test sees it: where the hub is and how big the wheel's mesh is.
fn tyre_volume(vehicle: &Vehicle, wheel: usize) -> Option<TyreVolume> {
    let hub = vehicle.wheels.get(wheel)?.position;
    let (lateral_in, lateral_out) = extent(vehicle, wheel, 0)?;
    let (low, high) = extent(vehicle, wheel, 1)?;
    Some(TyreVolume {
        lateral: hub[0].abs(),
        height: hub[1],
        radius: (high - low) / 2.0,
        half_width: (lateral_out - lateral_in) / 2.0,
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
    let left = |pair: [usize; 2]| pair.into_iter().find(|&wheel| vehicle.wheels[wheel].position[0] > 0.0);
    let (front_wheel, rear_wheel) = (left([order[0], order[1]])?, left([order[2], order[3]])?);
    let mut axles = hubs;
    for (end, axle) in axles.iter_mut().enumerate() {
        let wheel = [front_wheel, rear_wheel][end];
        let side = if end == 0 { "front" } else { "rear" };
        if !plausible(*axle, end == 0, body.0, body.1) {
            if !plausible(info_axles[end], end == 0, body.0, body.1) {
                return None;
            }
            log::warn!("the hubs of the {side} axle lie outside the body: the vehicle info's axle is used");
            *axle = info_axles[end];
            continue;
        }
        let Some(volume) = tyre_volume(vehicle, wheel) else { continue };
        let Some(lod) = vehicle.body.first() else { continue };
        if plausible(info_axles[end], end == 0, body.0, body.1) && prefer_other(lod, &volume, *axle, info_axles[end]) {
            log::warn!("the body has no arch at the hubs of the {side} axle: the vehicle info's axle is used");
            *axle = info_axles[end];
        }
    }
    Some(Placed {
        layout: measure_on(vehicle, Some(axles))?,
        front: tyre(vehicle, front_wheel, axles[0])?,
        rear: tyre(vehicle, rear_wheel, axles[1])?,
        axles,
    })
}

impl Placed {
    /// Move the hubs of `vehicle` onto the axles this layout uses, so what is built from the hubs afterwards (the tyres'
    /// collision boxes) agrees with the `[wheels]` table where an axle was taken from the vehicle info.
    pub fn seat_hubs(&self, vehicle: &mut Vehicle) {
        let middle = (self.axles[0] + self.axles[1]) / 2.0;
        for wheel in &mut vehicle.wheels {
            let end = usize::from(to_car_space(wheel.position)[0] <= middle);
            wheel.position[2] = self.axles[end];
        }
    }

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

    fn mesh(points: &[[f32; 3]]) -> chameleon_vehicle::PartMesh {
        chameleon_vehicle::PartMesh {
            vertices: points
                .iter()
                .map(|&position| chameleon_gfx::Vertex { position, color: [255; 4], uv: [0.0; 2] })
                .collect(),
            triangles: Vec::new(),
            appearance: chameleon_vehicle::Appearance {
                kind: chameleon_vehicle::SurfaceKind::Body,
                technique: String::new(),
                diffuse_texture: None,
                diffuse_colour: None,
                alpha_test: false,
            },
        }
    }

    /// A car whose body has a panel lying across the tyre at each of `panels` (z along the car), and whose hubs are at
    /// 1.4 and -1.2.
    fn car_with_panels(panels: &[f32]) -> Vehicle {
        let tyre = vec![vec![mesh(&[[0.12, 0.0, 0.33], [-0.12, 0.0, -0.33], [0.0, 0.33, 0.0], [0.0, -0.33, 0.0]])]];
        let wheel = |x: f32, z: f32| chameleon_vehicle::Wheel {
            name: String::new(),
            position: [x, 0.33, z],
            lods: tyre.clone(),
        };
        let panel = |z: f32| (0..40).map(move |n| [0.76, 0.33 + 0.001 * n as f32, z]);
        let points: Vec<[f32; 3]> = panels.iter().flat_map(|&z| panel(z)).collect();
        Vehicle {
            body: vec![vec![mesh(&points)]],
            wheels: vec![wheel(0.76, 1.4), wheel(-0.76, 1.4), wheel(0.76, -1.2), wheel(-0.76, -1.2)],
        }
    }

    #[test]
    fn an_axle_whose_hubs_are_inside_the_body_goes_to_where_the_body_has_its_arch() {
        // Panels where the hubs are, none at the vehicle info's axles (1.0 and -0.9): the body's arches are there.
        let placed = place(&car_with_panels(&[1.4, -1.2]), (-1.8, 1.8), [1.0, -0.9]).unwrap();
        assert_eq!(placed.axles, [1.0, -0.9]);
        assert_eq!(placed.front.x, 1.0);
    }

    #[test]
    fn hubs_with_room_in_the_body_stay_where_the_spec_puts_them() {
        let placed = place(&car_with_panels(&[]), (-1.8, 1.8), [1.0, -0.9]).unwrap();
        assert_eq!(placed.axles, [1.4, -1.2]);
        let blocked_both = place(&car_with_panels(&[1.4, -1.2, 1.0, -0.9]), (-1.8, 1.8), [1.0, -0.9]).unwrap();
        assert_eq!(blocked_both.axles, [1.4, -1.2], "no arch at either place: the spec's hubs");
    }

    #[test]
    fn the_hubs_are_seated_on_the_axles() {
        let wheel = |position: [f32; 3]| chameleon_vehicle::Wheel { name: String::new(), position, lods: Vec::new() };
        let mut vehicle = Vehicle {
            body: Vec::new(),
            wheels: vec![
                wheel([0.76, 0.3, 1.42]),
                wheel([-0.76, 0.3, 1.42]),
                wheel([0.76, 0.3, -1.24]),
                wheel([-0.76, 0.3, -1.24]),
            ],
        };
        let tyre = Tyre { x: 1.0, z: 0.3, face: 0.9, radius: 0.33, width: 0.25 };
        let layout = Layout { wheelbase: 1.96, x_mid: 0.05, track: 0.9, hub_z: 0.3, radius: 0.33 };
        Placed { layout, front: tyre, rear: tyre, axles: [1.035, -0.925] }.seat_hubs(&mut vehicle);
        let seated: Vec<f32> = vehicle.wheels.iter().map(|wheel| wheel.position[2]).collect();
        assert_eq!(seated, [1.035, 1.035, -0.925, -0.925]);
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
