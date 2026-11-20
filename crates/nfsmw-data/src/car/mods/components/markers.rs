//! The tail pipes of a fitted exhaust: the flames start where the part that is fitted ends, not where the stock one did.
//!
//! The game lights a car's tail pipes at the markers `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST` of the solids it draws
//! (`car::exhaust::pipe_frames`), and a bike's stock silencer is in the body, whose markers the importer wrote at its outlet. An
//! aftermarket exhaust (`[parts."exhaust.muffler.<id>"]`) is another solid mounted elsewhere on the bike (a top-mounted one is a
//! metre above the stock pipe), so its binding says where its outlet is (`markers`), in the car's frame, the one its `matrix`
//! takes the solid to: the importer finds the outlet in the silencer's mesh after the mount
//! (`mods/ride6-bikes/src/exhaust.rs`, `options_pack.rs`). When a part like that is drawn:
//!
//! - the pipe markers of every solid of the model (the stock exhaust's) are taken off ([`strip_pipes`]), and
//! - the part's own markers are written on its mounted copy ([`pipe_markers`]), which sits in the body's frame, so
//!   `pipe_frames` finds them with the flames, pops and nitrous emitters.
//!
//! A twin silencer is two parts (one per can) with a marker each, so both cans flame. A part with no markers (an exhaust
//! imported before they were written, or a stock one) leaves the stock pipes as they were.

use std::collections::HashMap;

use blackbox_hash::bstring_hash;
use blackbox_solid::{PositionMarker, Solid};
use glam::Vec3;

use super::{Binding, PartMarker, Role};
use crate::car::exhaust::PIPE_MARKERS;

/// Whether a part's markers name at least one tail pipe and the part sits in the body's frame, so that it sets the car's pipes.
pub fn sets_pipes(binding: &Binding) -> bool {
    binding.role == Role::Body && binding.markers.iter().any(is_pipe)
}

fn is_pipe(marker: &PartMarker) -> bool {
    PIPE_MARKERS.contains(&marker.name.as_str())
}

/// Take the tail-pipe markers off every solid of `solids`.
pub fn strip_pipes(solids: &mut HashMap<u32, Solid>) {
    let names = PIPE_MARKERS.map(bstring_hash);
    for solid in solids.values_mut() {
        solid.markers.retain(|marker| !names.contains(&marker.name_hash));
    }
}

/// The tail-pipe markers of `binding` as the solid format has them (none for a part that is not in the body's frame).
pub fn pipe_markers(binding: &Binding) -> Vec<PositionMarker> {
    if binding.role != Role::Body {
        return Vec::new();
    }
    binding.markers.iter().filter(|marker| is_pipe(marker)).map(marker).collect()
}

/// The marker of a pipe: its z axis is the way the flame shoots, its x axis points up (forward for a pipe that shoots up) and its
/// y axis completes a right-handed frame, the frame the stock markers have. Stored row-major: the three axes, then the position.
pub fn marker(pipe: &PartMarker) -> PositionMarker {
    let z = Vec3::from(pipe.direction).normalize_or(Vec3::NEG_X);
    let reference = if z.z.abs() > 0.99 { Vec3::X } else { Vec3::Z };
    let x = (reference - z * reference.dot(z)).normalize_or(Vec3::X);
    let y = z.cross(x);
    let row = |v: Vec3, w: f32| [v.x, v.y, v.z, w];
    let [px, py, pz] = pipe.position;
    let mut matrix = [0.0; 16];
    for (k, part) in [row(x, 0.0), row(y, 0.0), row(z, 0.0), [px, py, pz, 1.0]].into_iter().enumerate() {
        matrix[k * 4..k * 4 + 4].copy_from_slice(&part);
    }
    PositionMarker { name_hash: bstring_hash(&pipe.name), int_param: 0, float_params: [0.0; 2], matrix }
}

#[cfg(test)]
#[path = "markers_tests.rs"]
mod tests;
