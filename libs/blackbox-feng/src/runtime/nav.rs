//! Moving the focus between buttons by geometry (spec `docs/specs/feng-input.md`, section 3).

use glam::Vec2;

use super::{ObjectRef, PackageId, Runtime};

/// The eight directions as screen vectors (y down): up, up-right, right, down-right, down, down-left, left, up-left.
const DIRECTIONS: [Vec2; 8] = [
    Vec2::new(0.0, -1.0),
    Vec2::new(D, -D),
    Vec2::new(1.0, 0.0),
    Vec2::new(D, D),
    Vec2::new(0.0, 1.0),
    Vec2::new(-D, D),
    Vec2::new(-1.0, 0.0),
    Vec2::new(-D, -D),
];

/// One component of a diagonal unit vector (the game stores 0.70711).
const D: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// Score of a button that lies off the direction; anything at or above it is not a candidate.
const NO_CANDIDATE: f32 = 1500.0;

/// The best button from `from` towards `direction` among `candidates` (object index and screen position).
pub(super) fn best_button(from: Vec2, direction: usize, candidates: &[(usize, Vec2)]) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for &(index, at) in candidates {
        let delta = at - from;
        let distance = delta.length();
        if distance < 0.0001 {
            continue;
        }
        let mut angle = (delta / distance).dot(DIRECTIONS[direction]);
        if angle >= 0.0 {
            angle *= angle;
        }
        let score = if angle >= 0.25 { (1.0 - angle) * 200.0 + distance } else { NO_CANDIDATE };
        if score < best.map_or(NO_CANDIDATE, |b| b.1) {
            best = Some((index, score));
        }
    }
    best.map(|b| b.0)
}

impl Runtime {
    /// The button object index a direction leads to from `button`, or none.
    pub(super) fn button_from(&self, package: PackageId, button: usize, direction: usize) -> Option<usize> {
        let p = self.running(package)?;
        let tree = self.tree(package);
        let position = |i: usize| tree.nodes.iter().find(|n| n.index == i).map(|n| n.position.truncate());
        let from = position(button)?;
        let candidates: Vec<(usize, Vec2)> =
            p.buttons.iter().filter(|&&b| b != button).filter_map(|&b| Some((b, position(b)?))).collect();
        best_button(from, direction, &candidates)
    }

    /// The current button of a package.
    pub fn current_button(&self, package: PackageId) -> Option<ObjectRef> {
        let index = self.running(package)?.current_button?;
        Some(ObjectRef { package, index })
    }

    /// The buttons of a package (objects flagged as buttons), in file order.
    pub fn buttons(&self, package: PackageId) -> Vec<ObjectRef> {
        let Some(p) = self.running(package) else { return Vec::new() };
        p.buttons.iter().map(|&index| ObjectRef { package, index }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_button_in_the_direction_wins() {
        let row = [(1, Vec2::new(-70.0, 0.0)), (2, Vec2::new(70.0, 0.0)), (3, Vec2::new(140.0, 0.0))];
        let from = Vec2::ZERO;
        assert_eq!(best_button(from, 2, &row), Some(2), "right");
        assert_eq!(best_button(from, 6, &row), Some(1), "left");
        assert_eq!(best_button(from, 0, &row), None, "nothing above");
        assert_eq!(best_button(from, 4, &row), None, "nothing below");
    }

    #[test]
    fn a_button_off_axis_loses_to_one_on_axis() {
        let candidates = [(1, Vec2::new(60.0, 40.0)), (2, Vec2::new(90.0, 0.0))];
        assert_eq!(best_button(Vec2::ZERO, 2, &candidates), Some(2));
    }

    #[test]
    fn buttons_at_the_same_spot_are_skipped() {
        assert_eq!(best_button(Vec2::ZERO, 2, &[(1, Vec2::ZERO)]), None);
    }
}
