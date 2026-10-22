//! The grid: where each racer of a race starts, behind the player's place on the start line.
//! Spec: `docs/specs/custom-race.md` section 4 ("A race of sixteen").

use glam::Vec3;
use nfsmw_data::gameplay::{Placement, world_heading};

/// Metres between two rows of the grid, and between its two columns **[unconfirmed]**.
const ROW_GAP: f32 = 7.5;
const COLUMN_GAP: f32 = 3.5;
/// Most racers in a race, the player included: sixteen **[ours]**; the original's grid is four **[decomp]**. [`grid_slot`]
/// holds them in eight rows of two behind the player.
pub const MAX_RACERS: u8 = 16;

/// The grid slot `index` (0 is the player's): two columns, a row behind per two cars.
pub fn grid_slot(start: &Placement, index: usize) -> Placement {
    let forward = world_heading(start.heading_deg);
    let side = Vec3::Y.cross(forward);
    let (row, column) = ((index / 2) as f32, (index % 2) as f32);
    Placement {
        position: start.position - forward * row * ROW_GAP + side * column * COLUMN_GAP,
        heading_deg: start.heading_deg,
    }
}

/// The heading of a placement as the world's spawn heading: radians from +X in render space.
pub fn spawn_heading(heading_deg: f32) -> f32 {
    let forward = world_heading(heading_deg);
    // Physics (x, y, z) is render (z, -x, y).
    (-forward.x).atan2(forward.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_has_two_columns_behind_the_start() {
        let start = Placement { position: Vec3::new(5.0, 1.0, 5.0), heading_deg: 0.0 };
        let player = grid_slot(&start, 0);
        assert_eq!(player.position, start.position);
        let beside = grid_slot(&start, 1);
        assert!((beside.position.z - 5.0).abs() < 1e-4 && (beside.position.x - 5.0).abs() > 3.0);
        let behind = grid_slot(&start, 2);
        assert!((behind.position.z - (5.0 - 7.5)).abs() < 1e-4, "{behind:?}");
        assert_eq!(behind.position.x, 5.0);
    }

    #[test]
    fn a_full_grid_of_sixteen_is_apart_and_behind_the_line() {
        let start = Placement { position: Vec3::new(5.0, 1.0, 5.0), heading_deg: 0.0 };
        let forward = world_heading(start.heading_deg);
        let slots: Vec<Vec3> = (0..usize::from(MAX_RACERS)).map(|i| grid_slot(&start, i).position).collect();
        for (i, slot) in slots.iter().enumerate().skip(1) {
            assert!((*slot - start.position).dot(forward) <= 1e-4, "racer {i} is in front of the line: {slot:?}");
        }
        for (i, a) in slots.iter().enumerate() {
            for b in &slots[i + 1..] {
                assert!(a.distance(*b) >= COLUMN_GAP - 1e-4, "{a:?} and {b:?} overlap");
            }
        }
    }

    #[test]
    fn the_spawn_heading_points_along_the_grid() {
        // Heading 0 is +z in physics, which is +x in render space: radians 0.
        assert!(spawn_heading(0.0).abs() < 1e-5);
        // Heading 90 is -x in physics, which is +y in render space.
        assert!((spawn_heading(90.0) - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    }
}
