//! The parts of the KTM 450 SMR 2022 and where the RIDE 6 skeleton puts them.
//!
//! The exports hold each part in its own frame (a wheel at the origin with its axle along z), and the bike's frame holds
//! the hubs: `BONE_F_PHY_WHEEL` and `BONE_B_PHY_WHEEL` of the chassis skeleton, and the brake and handlebar bones under
//! them. Positions are in the exports' frame (glTF: x forward, y up, z to the right), metres.

/// What a part is made of, which picks the shader and the colour of a surface that has no picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Body,
    Tyre,
    Rim,
    Brake,
    Metal,
}

pub struct Part {
    /// The export's file stem (`chassis369` is `chassis369.glb`, its levels of detail `chassis369_LOD2.glb`).
    pub stem: &'static str,
    pub kind: Kind,
    /// Where the part's origin sits on the bike.
    pub offset: [f32; 3],
    /// The chassis picks the level of detail of the body solids; the others follow it.
    pub chassis: bool,
}

pub const FRONT_HUB: [f32; 3] = [0.7355, 0.225, 0.0];
pub const REAR_HUB: [f32; 3] = [-0.739, 0.232, 0.0];

pub const PARTS: [Part; 10] = [
    Part { stem: "chassis369", kind: Kind::Body, offset: [0.0, 0.0, 0.0], chassis: true },
    Part { stem: "f_rim369", kind: Kind::Rim, offset: FRONT_HUB, chassis: false },
    Part { stem: "f_tyre003", kind: Kind::Tyre, offset: FRONT_HUB, chassis: false },
    Part { stem: "frontbrake369", kind: Kind::Brake, offset: [FRONT_HUB[0], FRONT_HUB[1], -0.063], chassis: false },
    Part { stem: "b_rim369", kind: Kind::Rim, offset: REAR_HUB, chassis: false },
    Part { stem: "b_tyre002", kind: Kind::Tyre, offset: REAR_HUB, chassis: false },
    Part { stem: "backbrake369", kind: Kind::Brake, offset: [REAR_HUB[0], REAR_HUB[1], 0.071], chassis: false },
    Part { stem: "muffler369", kind: Kind::Metal, offset: [0.0, 0.0, 0.0], chassis: false },
    Part { stem: "exmanifold369_369", kind: Kind::Metal, offset: [0.0, 0.0, 0.0], chassis: false },
    Part { stem: "barpad369_369", kind: Kind::Body, offset: [0.28, 1.065, 0.0], chassis: false },
];

/// The solid letters and the export level of detail each takes: the chassis, then every other part. The finest body the
/// game's solids hold is 60,000 vertices, which level 2 of this chassis fits with the parts at level 1.
pub const LEVELS: [(char, u32, u32); 4] = [('A', 2, 1), ('B', 2, 1), ('C', 4, 3), ('D', 6, 5)];

/// The export frame's point as a point of the car's frame (x forward, y left, z up), the bike moved so the middle of its
/// wheelbase is the origin and the tyres touch the ground. `lift` is how far the lowest tyre point is below the ground.
pub fn to_car([x, y, z]: [f32; 3], centre: f32, lift: f32) -> [f32; 3] {
    [x - centre, -z, y + lift]
}

/// A direction, turned the same way (no move).
pub fn direction_to_car([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, -z, y]
}

/// The middle of the wheelbase along x.
pub fn wheelbase_centre() -> f32 {
    (FRONT_HUB[0] + REAR_HUB[0]) / 2.0
}

pub fn wheelbase() -> f32 {
    FRONT_HUB[0] - REAR_HUB[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_right_side_is_minus_y_and_up_is_z() {
        assert_eq!(to_car([1.0, 2.0, 3.0], 0.5, 0.1), [0.5, -3.0, 2.1]);
        assert_eq!(direction_to_car([0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn the_wheelbase_is_the_hub_distance() {
        assert!((wheelbase() - 1.4745).abs() < 1e-4);
        assert!(wheelbase_centre().abs() < 0.01);
    }
}
