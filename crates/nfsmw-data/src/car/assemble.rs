//! From slots to placed solids (docs/specs/car-assembly.md §2–5).

use std::collections::HashMap;
use std::sync::Arc;

use blackbox_carparts::Part;
use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use glam::Mat4;

use super::ecar::{BikeMount, WheelSetup};
use super::stock::{Slots, kit_number};
use super::tables::{CarTables, slot};
use super::wheels::{self, Corner, WheelModel};

/// One solid drawn at a transform in car space.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// The solid's name hash.
    pub solid: u32,
    pub transform: Mat4,
    /// A left brake: draws `BRAKE_GLOBAL_LEFT` instead of `BRAKE_GLOBAL`.
    pub left_brake: bool,
    /// The slot it came from.
    pub slot: usize,
    /// The wheel (0 = front left, 1 = front right, 2 = rear right, 3 = rear left) a wheel or brake
    /// belongs to; `None` for the rest of the car.
    pub corner: Option<usize>,
}

impl Placement {
    /// A brake disc and caliper (as opposed to a wheel or a body part).
    pub fn is_brake(&self) -> bool {
        self.slot == slot::FRONT_BRAKE || self.slot == slot::REAR_BRAKE
    }
}

/// Whether a model slot is drawn in the car's body frame. The driver is drawn only when driving.
fn drawn_in_body_frame(s: usize, slots: &Slots, driver: bool) -> bool {
    match s {
        slot::DRIVER => driver,
        slot::LICENSE_PLATE => false,
        slot::FRONT_WHEEL | slot::REAR_WHEEL | slot::FRONT_BRAKE | slot::REAR_BRAKE => false,
        slot::UNIVERSAL_SPOILER_BASE => slots[slot::SPOILER].is_some_and(|p| p.upgrade_level != 0),
        // Corner damage is placed but the renderer keeps it hidden until its zone is damaged
        // (`CarModel::damage_overlay`); decal models only show transparent DEFAULTALPHA without a decal.
        s if slot::DECAL_MODELS.contains(&s) => {
            let start = *slot::DECAL_TEXTURES.start() + (s - slot::DECAL_MODELS.start()) * 8;
            slots[start..start + 8].iter().any(Option::is_some)
        }
        _ => true,
    }
}

/// Place every drawn part. `solids` holds every solid that may be referenced, by name hash.
pub fn assemble(
    t: &CarTables,
    slots: &Slots,
    solids: &HashMap<u32, Arc<Solid>>,
    setup: Option<&WheelSetup>,
    lod: usize,
    driver: bool,
) -> (Vec<Placement>, Option<[Corner; 4]>) {
    let model = |part: &Option<Part>| part.and_then(|p| t.parts.model_hash(&p, lod)).filter(|h| solids.contains_key(h));
    let base = model(&slots[slot::BASE]).and_then(|h| solids.get(&h)).map(Arc::as_ref);
    let mut out = Vec::new();

    for s in (0..slot::MODELS).filter(|&s| drawn_in_body_frame(s, slots, driver)) {
        let Some(part) = slots[s] else { continue };
        let Some(hash) = model(&slots[s]) else { continue };
        let transform = match (s, part.upgrade_level) {
            // Aftermarket spoilers and roof scoops sit at markers on the base solid.
            (slot::SPOILER, 1..) => {
                let marker =
                    if t.parts.attribute(&part, "USEMARKER2").is_some_and(|v| v != 0) { "SPOILER2" } else { "SPOILER" };
                marker_transform(base, marker)
            }
            (slot::ROOF, 1..) => marker_transform(base, "ROOF_SCOOP"),
            _ => Some(Mat4::IDENTITY),
        };
        if let Some(transform) = transform {
            out.push(Placement { solid: hash, transform, left_brake: false, slot: s, corner: None });
        }
    }

    let (Some(setup), Some(wheel)) = (setup, model(&slots[slot::FRONT_WHEEL])) else { return (out, None) };
    if let Some(mount) = &setup.bike {
        let rear = model(&slots[slot::REAR_WHEEL]).unwrap_or(wheel);
        let (fork, bars) = (model(&slots[slot::FRONT_BRAKE]), model(&slots[slot::REAR_BRAKE]));
        let corners = bike_wheels(setup, mount, ([wheel, rear], [fork, bars]), solids, &mut out);
        return (out, Some(corners));
    }
    let wheel_solid = &solids[&wheel];
    let marker_y = |name: &str| wheel_solid.marker(bstring_hash(name)).map(|m| m.translation()[1]);
    let front_marker = marker_y("FRONT_BRAKE").unwrap_or(0.0);
    let markers = [front_marker, marker_y("REAR_BRAKE").unwrap_or(front_marker)];
    let kit = slots[slot::BODY].map_or(0, |b| kit_number(t, &b)) as usize;
    let size = WheelModel::from_bounds(wheel_solid.bounds_min, wheel_solid.bounds_max);
    let size = if size.width > 0.0 && size.radius > 0.0 { size } else { WheelModel::FALLBACK };
    let front_brake = model(&slots[slot::FRONT_BRAKE]);
    let brakes = [front_brake, model(&slots[slot::REAR_BRAKE]).or(front_brake)];

    let corners = wheels::place(setup, kit, size, markers, lod <= 1);
    for (i, corner) in corners.iter().enumerate() {
        let wheel_slot = if i < 2 { slot::FRONT_WHEEL } else { slot::REAR_WHEEL };
        out.push(Placement {
            solid: wheel,
            transform: corner.wheel,
            left_brake: false,
            slot: wheel_slot,
            corner: Some(i),
        });
        if let Some(brake) = brakes[usize::from(i >= 2)] {
            let brake_slot = if i < 2 { slot::FRONT_BRAKE } else { slot::REAR_BRAKE };
            out.push(Placement {
                solid: brake,
                transform: corner.brake,
                left_brake: corner.left,
                slot: brake_slot,
                corner: Some(i),
            });
        }
    }
    (out, Some(corners))
}

/// The two wheels of a bike: the front wheel in corner 0 and the rear wheel in corner 2, each its own solid with its axle
/// along y and its hub at the origin. A bike draws no brake parts of a car's kind (its discs are part of its wheel solids)
/// and nothing on corner 3. `models` are the front and the rear wheel solid. `front_end` are the solids of the front end that
/// moves, if the bike has them, both modelled with the front hub on their origin and placed as a wheel's brake is
/// (`Corner::posed`, no spin): the fork (legs, fender, caliper) in the front brake slot with corner 0, which steers about
/// the steering axis and travels along it with the wheel, and the bars (handlebar, levers, grips, headlamp) in the rear
/// brake slot with corner 1, which steers with the fork and does not travel. Without them the front end is in the body.
fn bike_wheels(
    setup: &WheelSetup,
    mount: &BikeMount,
    (models, front_end): ([u32; 2], [Option<u32>; 2]),
    solids: &HashMap<u32, Arc<Solid>>,
    out: &mut Vec<Placement>,
) -> [Corner; 4] {
    let size = |hash: u32| {
        let solid = &solids[&hash];
        let size = WheelModel::from_bounds(solid.bounds_min, solid.bounds_max);
        match size.width > 0.0 && size.radius > 0.0 {
            true => size,
            false => WheelModel::FALLBACK,
        }
    };
    let corners = wheels::place_bike(setup, mount, size(models[0]), size(models[1]));
    for (corner, slot, hash) in [(0, slot::FRONT_WHEEL, models[0]), (2, slot::REAR_WHEEL, models[1])] {
        out.push(Placement {
            solid: hash,
            transform: corners[corner].wheel,
            left_brake: false,
            slot,
            corner: Some(corner),
        });
    }
    for (solid, corner, slot) in [(front_end[0], 0, slot::FRONT_BRAKE), (front_end[1], 1, slot::REAR_BRAKE)] {
        if let Some(solid) = solid {
            out.push(Placement {
                solid,
                transform: corners[corner].brake,
                left_brake: false,
                slot,
                corner: Some(corner),
            });
        }
    }
    corners
}

/// A marker's matrix as a transform (stored row-major for row vectors, i.e. the transpose of
/// glam's column-major layout).
fn marker_transform(base: Option<&Solid>, name: &str) -> Option<Mat4> {
    base?.marker(bstring_hash(name)).map(|m| Mat4::from_cols_array(&m.matrix))
}
