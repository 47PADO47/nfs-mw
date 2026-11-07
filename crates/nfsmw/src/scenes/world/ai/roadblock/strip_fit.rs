//! **[ours]** Keeping a spike strip on the road it blocks (`docs/specs/ai-pursuit-roadblocks.md` section 2.5). A layout
//! slot near the road's edge put part of a 6.4 m strip over the kerb or into the railing beside it, and the strip's
//! seat then tilted it up onto what lay there. The strip is moved along the road's width until both ends are inside its
//! edges, and the seat ignores a surface at an end that breaks from the road's slope (`build.rs` `fitted_surface`).

use blackbox_pursuit::roadblock::Road;
use glam::{Vec2, Vec3};

/// How far inside the road's edges a strip's ends stay, metres.
const EDGE_MARGIN: f32 = 0.2;
/// A surface at a strip's end this far above or below the road's slope (the line from its centre through the point
/// halfway out) belongs to something beside the road (a kerb, a railing's foot), metres.
pub(super) const MAX_END_STEP: f32 = 0.12;

/// Where a strip centred at `position`, facing `heading` (radians in the physics x-z plane) with its long axis across its
/// facing and `half_length` along that axis, lies with both ends inside the road's edges: moved along the road's width as
/// little as needed, or centred on the road when it is longer than the road is wide. The height is kept.
pub(super) fn inside_road(position: Vec3, heading: f32, half_length: f32, road: &Road) -> Vec3 {
    let flat = |v: Vec3| Vec2::new(v.x, v.z);
    let (left, right) = (flat(road.left), flat(road.right));
    let width = left.distance(right);
    let Some(across) = (right - left).try_normalize() else { return position };
    let axis = Vec2::new(-heading.sin(), heading.cos());
    let centre = flat(position);
    let reach = (axis * half_length).dot(across).abs();
    let at = (centre - left).dot(across);
    let (low, high) = (reach + EDGE_MARGIN, width - reach - EDGE_MARGIN);
    let wanted = match low <= high {
        true => at.clamp(low, high),
        false => width / 2.0,
    };
    let moved = centre + across * (wanted - at);
    Vec3::new(moved.x, position.y, moved.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 10 m road along +z: its left edge at x 0, its right at x 10.
    fn road() -> Road {
        Road { left: Vec3::ZERO, right: Vec3::new(10.0, 0.0, 0.0), target: Vec3::ZERO, forward: Vec3::Z }
    }

    #[test]
    fn a_strip_hanging_over_an_edge_moves_back_onto_the_road() {
        // Facing +z, the long axis runs along x: the strip spans x 7..13 at x 10, past the right edge.
        let moved = inside_road(Vec3::new(10.0, 2.0, 5.0), std::f32::consts::FRAC_PI_2, 3.0, &road());
        assert!((moved.x - 6.8).abs() < 1e-4, "{moved}");
        assert_eq!((moved.y, moved.z), (2.0, 5.0));
    }

    #[test]
    fn a_strip_inside_the_road_stays_and_one_longer_than_the_road_is_centred() {
        let inside = Vec3::new(5.0, 0.0, 0.0);
        assert_eq!(inside_road(inside, std::f32::consts::FRAC_PI_2, 3.0, &road()), inside);
        let long = inside_road(Vec3::new(2.0, 0.0, 0.0), std::f32::consts::FRAC_PI_2, 6.0, &road());
        assert!((long.x - 5.0).abs() < 1e-4);
    }

    #[test]
    fn a_strip_lying_along_the_road_needs_no_room_across_it() {
        // Facing +x, its long axis runs along the road: only its width side matters, so it may stand near the edge.
        let near_edge = Vec3::new(9.5, 0.0, 0.0);
        assert_eq!(inside_road(near_edge, 0.0, 3.0, &road()), near_edge);
    }
}
