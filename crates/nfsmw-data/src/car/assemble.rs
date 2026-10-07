//! From slots to placed solids (docs/specs/car-assembly.md §2–5).

use std::collections::HashMap;

use blackbox_carparts::Part;
use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use glam::Mat4;

use super::ecar::WheelSetup;
use super::stock::{Slots, kit_number};
use super::tables::{CarTables, slot};
use super::wheels::{self, WheelModel};

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
}

/// Whether a model slot is drawn in the front end, where it isn't placed specially.
fn drawn_in_body_frame(s: usize, slots: &Slots) -> bool {
    match s {
        slot::DRIVER | slot::LICENSE_PLATE => false,
        slot::FRONT_WHEEL | slot::REAR_WHEEL | slot::FRONT_BRAKE | slot::REAR_BRAKE => false,
        slot::UNIVERSAL_SPOILER_BASE => slots[slot::SPOILER].is_some_and(|p| p.upgrade_level != 0),
        // Corner damage stays hidden until damaged; decal models only show transparent
        // DEFAULTALPHA without a decal.
        s if slot::DAMAGE0.contains(&s) || slot::DECAL_MODELS.contains(&s) => false,
        _ => true,
    }
}

/// Place every drawn part. `solids` holds every solid that may be referenced, by name hash.
pub fn assemble(
    t: &CarTables,
    slots: &Slots,
    solids: &HashMap<u32, Solid>,
    setup: Option<&WheelSetup>,
    lod: usize,
) -> Vec<Placement> {
    let model = |part: &Option<Part>| part.and_then(|p| t.parts.model_hash(&p, lod)).filter(|h| solids.contains_key(h));
    let base = model(&slots[slot::BASE]).and_then(|h| solids.get(&h));
    let mut out = Vec::new();

    for s in (0..slot::MODELS).filter(|&s| drawn_in_body_frame(s, slots)) {
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
            out.push(Placement { solid: hash, transform, left_brake: false, slot: s });
        }
    }

    let (Some(setup), Some(wheel)) = (setup, model(&slots[slot::FRONT_WHEEL])) else { return out };
    let wheel_solid = &solids[&wheel];
    let marker_y = |name: &str| wheel_solid.marker(bstring_hash(name)).map(|m| m.translation()[1]);
    let front_marker = marker_y("FRONT_BRAKE").unwrap_or(0.0);
    let markers = [front_marker, marker_y("REAR_BRAKE").unwrap_or(front_marker)];
    let kit = slots[slot::BODY].map_or(0, |b| kit_number(t, &b)) as usize;
    let size = WheelModel::from_bounds(wheel_solid.bounds_min, wheel_solid.bounds_max);
    let size = if size.width > 0.0 && size.radius > 0.0 { size } else { WheelModel::FALLBACK };
    let front_brake = model(&slots[slot::FRONT_BRAKE]);
    let brakes = [front_brake, model(&slots[slot::REAR_BRAKE]).or(front_brake)];

    for (i, corner) in wheels::place(setup, kit, size, markers, lod <= 1).into_iter().enumerate() {
        let wheel_slot = if i < 2 { slot::FRONT_WHEEL } else { slot::REAR_WHEEL };
        out.push(Placement { solid: wheel, transform: corner.wheel, left_brake: false, slot: wheel_slot });
        if let Some(brake) = brakes[usize::from(i >= 2)] {
            let brake_slot = if i < 2 { slot::FRONT_BRAKE } else { slot::REAR_BRAKE };
            out.push(Placement { solid: brake, transform: corner.brake, left_brake: corner.left, slot: brake_slot });
        }
    }
    out
}

/// A marker's matrix as a transform (stored row-major for row vectors, i.e. the transpose of
/// glam's column-major layout).
fn marker_transform(base: Option<&Solid>, name: &str) -> Option<Mat4> {
    base?.marker(bstring_hash(name)).map(|m| Mat4::from_cols_array(&m.matrix))
}
