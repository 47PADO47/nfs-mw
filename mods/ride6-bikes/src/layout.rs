//! The car frame and the levels of detail.
//!
//! The exports hold a bike in glTF axes (x forward, y up, z to the right), metres, on the chassis skeleton of the
//! engine: the origin is the skeleton's base, not the ground. The car's frame is x forward, y left, z up
//! ([car-customization.md](../../../docs/modding/car-customization.md#the-files-of-a-car)). A body solid has the middle of
//! the wheelbase on the origin and the lowest tyre point on the ground; a wheel solid has the hub on the origin, its axle
//! along y, and sits on the plane y = 0 of the car frame.

/// The solid letters and the finest export level of detail of the chassis each body may use. A body takes the finest
/// level from this number on whose vertices fit one solid (the most the game's solids hold is 60,000; the full-detail
/// chassis has 135,000, level 1 has 78,000, level 2 44,000), so A and B are level 2 and C and D are level 3. A wheel
/// solid takes the export level of its letter's position (A is 0, D is 3): its meshes are small.
pub const LEVELS: [(char, u32); 4] = [('A', 0), ('B', 2), ('C', 3), ('D', 3)];

/// The deepest level of detail of the chassis the importer reads: the engine builds levels 4 and up from a composite
/// of its own at run time (one material for the whole bike), which has no export.
pub const COARSEST_CHASSIS_LEVEL: u32 = 3;

/// Where the bike sits in the car's frame: the middle of the wheelbase on the origin, the tyres on the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The middle of the wheelbase along the export x axis.
    pub centre: f32,
    /// How far the lowest tyre point is below the export origin's height.
    pub lift: f32,
}

impl Frame {
    /// A point of the chassis frame as a point of the body solid.
    pub fn body_point(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        [x - self.centre, -z, y + self.lift]
    }
}

/// A point of the chassis frame as a point of the wheel solid whose hub is at `hub`.
pub fn wheel_point([x, y, z]: [f32; 3], hub: [f32; 3]) -> [f32; 3] {
    [x - hub[0], -(z - hub[2]), y - hub[1]]
}

/// A direction of the chassis frame, turned the same way (no move).
pub fn direction_to_car([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, -z, y]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_right_side_is_minus_y_and_up_is_z() {
        let frame = Frame { centre: 0.5, lift: 0.1 };
        assert_eq!(frame.body_point([1.0, 2.0, 3.0]), [0.5, -3.0, 2.1]);
        assert_eq!(direction_to_car([0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn a_wheel_point_is_relative_to_its_hub() {
        let hub = [0.7, 0.2, 0.0];
        assert_eq!(wheel_point(hub, hub), [0.0; 3]);
        // A disc on the left of the hub (export -z) is at car +y.
        let disc = wheel_point([0.7, 0.2, -0.0635], hub);
        assert!((disc[1] - 0.0635).abs() < 1e-6 && disc[0].abs() < 1e-6 && disc[2].abs() < 1e-6);
        // A point above the hub is above it in the car frame.
        assert!((wheel_point([0.7, 0.5, 0.0], hub)[2] - 0.3).abs() < 1e-6);
    }
}
