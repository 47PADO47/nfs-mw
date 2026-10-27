//! Where a mod car's model has its wheels (the `[wheels]` table of the car manifest, docs/modding/car-stats.md).
//!
//! The physics of a car come from its `[stats]`; the wheels are drawn where the model has its hubs, which `[stats]` does not
//! say exactly (it has the wheelbase and the track of the tyres, not the height of a hub above the model's ground, nor how far
//! the model's tyre reaches). A car that publishes no layout has its wheels drawn from its stats (`WheelSetup::from_stats`).

use std::sync::{PoisonError, RwLock};

/// One axle's left tyre in the car's space (x forward, y left, z up). The right tyre is its mirror.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tyre {
    /// How far forward of the car's origin the tyre's centre is.
    pub x: f32,
    /// Height of the tyre's centre.
    pub z: f32,
    /// Distance from the centre line to the tyre's outer face.
    pub face: f32,
    pub radius: f32,
    pub width: f32,
}

/// The tyres of a car, front and rear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelLayout {
    pub front: Tyre,
    pub rear: Tyre,
}

/// The cars with a wheel layout, by folder. Each publication leaks the new list, as the mod cars do.
static PUBLISHED: RwLock<&'static [(String, WheelLayout)]> = RwLock::new(&[]);

/// Register the wheel layout of the mod car in `folder`. A folder already registered keeps its first.
pub fn publish(folder: &str, layout: WheelLayout) {
    let mut published = PUBLISHED.write().unwrap_or_else(PoisonError::into_inner);
    if published.iter().any(|(known, _)| known.eq_ignore_ascii_case(folder)) {
        return;
    }
    let mut all: Vec<(String, WheelLayout)> = published.to_vec();
    all.push((folder.to_owned(), layout));
    *published = Box::leak(all.into_boxed_slice());
}

/// The wheel layout of the mod car in `folder`. `None` for a stock car and for a mod car that gives none.
pub fn of(folder: &str) -> Option<WheelLayout> {
    let published = *PUBLISHED.read().unwrap_or_else(PoisonError::into_inner);
    published.iter().find(|(known, _)| known.eq_ignore_ascii_case(folder)).map(|(_, layout)| *layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::car::WheelSetup;
    use crate::car::mods::CarStats;

    fn layout() -> WheelLayout {
        WheelLayout {
            front: Tyre { x: 1.4, z: 0.334, face: 0.93, radius: 0.321, width: 0.265 },
            rear: Tyre { x: -1.4, z: 0.356, face: 0.98, radius: 0.358, width: 0.345 },
        }
    }

    #[test]
    fn a_layout_puts_each_tyre_where_the_model_has_it() {
        let setup = WheelSetup::from_layout(&CarStats::default(), Some(&layout()));
        // Front left, front right, rear right, rear left.
        assert_eq!(setup.tire_offsets[0], [1.4, 0.93, 0.334, 0.321]);
        assert_eq!(setup.tire_offsets[1], [1.4, -0.93, 0.334, 0.321]);
        assert_eq!(setup.tire_offsets[2], [-1.4, -0.98, 0.356, 0.358]);
        assert_eq!(setup.tire_offsets[3], [-1.4, 0.98, 0.356, 0.358]);
        assert_eq!(setup.skid_width, [0.265, 0.265, 0.345, 0.345]);
    }

    #[test]
    fn without_a_layout_the_stats_place_the_wheels() {
        let stats = CarStats::default();
        assert_eq!(WheelSetup::from_layout(&stats, None), WheelSetup::from_stats(&stats));
    }

    #[test]
    fn a_published_layout_is_found_by_folder_in_any_case() {
        publish("LAYOUTCAR1", layout());
        assert_eq!(of("layoutcar1"), Some(layout()));
        assert_eq!(of("NOLAYOUT"), None);
    }
}
