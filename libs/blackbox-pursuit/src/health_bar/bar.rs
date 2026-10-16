//! One bar, ready to draw, and the back-to-front sort. Spec sections 11 and 13.3. The colours are linear RGBA with the
//! final alpha already applied.

use glam::Vec3;

use super::tier::Tier;

/// One bar for one cop this step.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    /// The cop's key, the tie-break of the sort.
    pub key: u32,
    /// The bar's centre in render space: the cop's position lifted along world up.
    pub centre: Vec3,
    /// The camera's right vector, which the quads are laid along.
    pub right: Vec3,
    /// View depth of the centre (`dot(centre - camera, forward)`). The sort draws larger depths first.
    pub depth: f32,
    /// Displayed health, 0..=1. 0 once the cop is destroyed.
    pub health: f32,
    /// Lagging health, 0..=1, never below `health`.
    pub lag: f32,
    pub tier: Tier,
    /// The final alpha: `BAR_ALPHA` times the distance fade times the death alpha.
    pub alpha: f32,
    /// The death flash is in its "on" half.
    pub flash_on: bool,
    pub destroyed: bool,
    /// The border colour, alpha included.
    pub border_rgba: [f32; 4],
    /// The fill colour (the tier), alpha included.
    pub fill_rgba: [f32; 4],
    /// The lagging segment colour, alpha included.
    pub lag_rgba: [f32; 4],
}

/// Sorts back to front: the farthest bar first. Equal depths go by `key`, so the order is the same every frame.
pub fn sort_back_to_front(bars: &mut [Bar]) {
    bars.sort_by(|a, b| b.depth.total_cmp(&a.depth).then(a.key.cmp(&b.key)));
}

/// A bar with plain values, for the tests of this module and of its siblings.
#[cfg(test)]
pub(super) fn test_bar(key: u32, depth: f32) -> Bar {
    Bar {
        key,
        centre: Vec3::ZERO,
        right: Vec3::X,
        depth,
        health: 1.0,
        lag: 1.0,
        tier: Tier::Green,
        alpha: 0.7,
        flash_on: false,
        destroyed: false,
        border_rgba: [0.0, 0.0, 0.0, 0.7],
        fill_rgba: [1.0, 1.0, 1.0, 0.7],
        lag_rgba: [1.0, 1.0, 1.0, 0.7],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(bars: &[Bar]) -> Vec<u32> {
        bars.iter().map(|bar| bar.key).collect()
    }

    #[test]
    fn farther_bar_comes_first() {
        let mut bars = vec![test_bar(1, 10.0), test_bar(2, 30.0), test_bar(3, 20.0)];
        sort_back_to_front(&mut bars);
        assert_eq!(keys(&bars), vec![2, 3, 1]);
    }

    #[test]
    fn equal_depths_go_by_key() {
        let mut bars = vec![test_bar(9, 15.0), test_bar(4, 15.0), test_bar(7, 15.0)];
        sort_back_to_front(&mut bars);
        assert_eq!(keys(&bars), vec![4, 7, 9]);
    }

    #[test]
    fn sort_is_in_place_and_keeps_every_bar() {
        let mut bars = vec![test_bar(1, 1.0), test_bar(2, 2.0)];
        sort_back_to_front(&mut bars);
        assert_eq!(bars.len(), 2);
        assert_eq!(keys(&bars), vec![2, 1]);
    }
}
