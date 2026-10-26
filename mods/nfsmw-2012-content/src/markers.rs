//! The markers a converted car carries, derived from its own body.
//!
//! The game starts the nitrous flames at the markers `LEFT_EXHAUST` and `RIGHT_EXHAUST` of the body solid. The 2012 files
//! have locators for the tail pipes, but their table is not decoded (`docs/formats/chameleon-customization.md`), so the
//! pipes are put where tail pipes sit on a car of this body: at the rear face, low, a little inside the corners. This is
//! an approximation made from the car's own shape; it is not the 2012 game's data.
//!
//! The 2012 body has no mount for an aftermarket spoiler or roof scoop (the game has none to offer), so none is made.

use blackbox_hash::bstring_hash;
use blackbox_solid::PositionMarker;

/// How far from the rear face a point still counts as the rear when the car's width there is measured, in metres.
const REAR_DEPTH: f32 = 0.4;
/// The pipes' lateral place as a fraction of the half width of the rear.
const PIPE_SPREAD: f32 = 0.5;
/// The pipes' height as a fraction of the car's height.
const PIPE_HEIGHT: f32 = 0.22;

/// The tail pipe markers of a body whose points, in car space (+x forward, +y left, +z up), are `body`. Empty for a body
/// with no points.
pub fn tail_pipes(body: &[[f32; 3]]) -> Vec<PositionMarker> {
    let Some(rear) = body.iter().map(|p| p[0]).reduce(f32::min) else { return Vec::new() };
    let (low, high) = body.iter().map(|p| p[2]).fold((f32::MAX, f32::MIN), |(low, high), z| (low.min(z), high.max(z)));
    let half_width = body.iter().filter(|p| p[0] <= rear + REAR_DEPTH).map(|p| p[1].abs()).fold(0.0, f32::max);
    let at = |side: f32| [rear, side * half_width * PIPE_SPREAD, low + PIPE_HEIGHT * (high - low)];
    [("LEFT_EXHAUST", 1.0), ("RIGHT_EXHAUST", -1.0)].into_iter().map(|(name, side)| marker(name, at(side))).collect()
}

/// A marker whose z axis points backwards, which is the way a tail pipe blows (the game's flames follow it).
fn marker(name: &str, at: [f32; 3]) -> PositionMarker {
    let mut matrix = [0.0; 16];
    matrix[0] = 1.0;
    matrix[5] = 1.0;
    matrix[8] = -1.0;
    matrix[15] = 1.0;
    matrix[12..15].copy_from_slice(&at);
    PositionMarker { name_hash: bstring_hash(name), int_param: 0, float_params: [0.0, 0.0], matrix }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A box 4 m long, 1.8 m wide, 1.2 m tall, standing on the ground, with its rear at x = -2.
    fn body() -> Vec<[f32; 3]> {
        let mut points = Vec::new();
        for x in [-2.0, 2.0] {
            for y in [-0.9, 0.9] {
                for z in [0.1, 1.3] {
                    points.push([x, y, z]);
                }
            }
        }
        points
    }

    #[test]
    fn the_pipes_sit_at_the_rear_face_low_and_either_side_of_the_centre() {
        let pipes = tail_pipes(&body());
        assert_eq!(pipes.len(), 2);
        let [left, right] = [pipes[0].translation(), pipes[1].translation()];
        assert_eq!(left[0], -2.0);
        assert!((left[1] - 0.45).abs() < 1e-5 && (right[1] + 0.45).abs() < 1e-5);
        assert!((left[2] - (0.1 + 0.22 * 1.2)).abs() < 1e-5);
        assert_eq!(pipes[0].name_hash, bstring_hash("LEFT_EXHAUST"));
    }

    #[test]
    fn a_pipe_blows_backwards() {
        let pipe = tail_pipes(&body())[0];
        assert_eq!(&pipe.matrix[8..11], &[-1.0, 0.0, 0.0]);
    }

    #[test]
    fn a_body_without_points_has_no_pipes() {
        assert!(tail_pipes(&[]).is_empty());
    }
}
