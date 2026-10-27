//! A layer the tests of this folder share.

use blackbox_modding::content::{SparkKind, SparkLayer};

/// A streak of 10 sparks that lives a second, flies at 8 m/s and shows 0.1 s of its travel, shrinking and fading over three keys.
pub(super) fn layer() -> SparkLayer {
    SparkLayer {
        kind: SparkKind::Streak,
        texture: None,
        additive: true,
        grid: 0,
        fps: 0,
        random_frame: false,
        count: 10.0,
        life: 1.0,
        life_variance: 0.0,
        speed: 8.0,
        speed_variance: 0.0,
        spread: 0.0,
        gravity: 0.0,
        drag: 0.0,
        bounce: None,
        friction: None,
        inherit: 0.0,
        inherit_variance: 0.0,
        stretch: 0.1,
        keys: vec![0.0, 0.5, 1.0],
        size: vec![0.1, 0.2, 0.0],
        colors: vec![[255, 100, 0, 255], [255, 200, 100, 128], [255, 255, 255, 0]],
        brightness: vec![10.0, 4.0, 1.0],
    }
}
