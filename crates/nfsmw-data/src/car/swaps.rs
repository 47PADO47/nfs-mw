//! Texture replacement (docs/specs/car-assembly.md §6): car solids name placeholder
//! textures that are swapped per car before drawing.

use std::collections::{HashMap, HashSet};

use blackbox_hash::bstring_hash;

/// Decal placeholders. With no decal chosen they show the fully transparent `DEFAULTALPHA`,
/// i.e. nothing, so their groups are hidden.
const DECAL_PLACEHOLDERS: &[&str] = &[
    "DUMMY_DECAL1",
    "DUMMY_DECAL2",
    "DUMMY_DECAL3",
    "DUMMY_DECAL4",
    "DUMMY_DECAL5",
    "DUMMY_DECAL6",
    "DUMMY_NUMBER_LEFT",
    "DUMMY_NUMBER_RIGHT",
    "BOTTOM_DECAL",
    "FRONT_BUMPER_DECAL",
    "FRONT_DECAL",
    "GTWING_DECAL",
    "HOOD_DECAL",
    "LEFT_BRAKELIGHT_DECAL",
    "LEFT_DOOR_DECAL",
    "LEFT_FENDER_DECAL",
    "LEFT_QUARTER_DECAL",
    "LEFT_SIDE_MIRROR_DECAL",
    "LEFT_SKIRT_DECAL",
    "REAR_BUMPER_DECAL",
    "REAR_DECAL",
    "RIGHT_BRAKELIGHT_DECAL",
    "RIGHT_DOOR_DECAL",
    "RIGHT_FENDER_DECAL",
    "RIGHT_QUARTER_DECAL",
    "RIGHT_SIDE_MIRROR_DECAL",
    "RIGHT_SKIRT_DECAL",
    "TOP_DECAL",
    "FRONT_WINDOW_DECAL",
    "REAR_WINDOW_DECAL",
    "LEFT_FRONT_WINDOW_DECAL",
    "LEFT_REAR_WINDOW_DECAL",
    "RIGHT_FRONT_WINDOW_DECAL",
    "RIGHT_REAR_WINDOW_DECAL",
];

/// Window placeholders; undamaged, they all show `WINDOW_FRONT`.
const WINDOWS: &[&str] = &[
    "WINDOW_FRONT",
    "WINDOW_REAR",
    "WINDOW_LEFT_FRONT",
    "WINDOW_LEFT_REAR",
    "WINDOW_RIGHT_FRONT",
    "WINDOW_RIGHT_REAR",
];

/// The brake caliper texture, and its mirrored-lettering version for left brakes.
const BRAKE: &str = "BRAKE_GLOBAL";
pub const BRAKE_LEFT: &str = "BRAKE_GLOBAL_LEFT";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureSwaps {
    /// Placeholder hash → texture hash to draw instead.
    pub map: HashMap<u32, u32>,
    /// Placeholders whose groups are not drawn.
    pub hidden: HashSet<u32>,
}

impl TextureSwaps {
    /// The swaps of a car with base model name `car` (stock lights: headlights on, brake
    /// lights off).
    pub fn for_car(car: &str) -> Self {
        let mut map = HashMap::new();
        let mut swap = |from: &str, to: &str| {
            map.insert(bstring_hash(from), bstring_hash(to));
        };
        let headlight = format!("{car}_KIT00_HEADLIGHT_ON");
        let brakelight = format!("{car}_KIT00_BRAKELIGHT_OFF");
        let brakelight_glass = format!("{car}_KIT00_BRAKELIGHT_GLASS_OFF");
        for side in ["LEFT", "RIGHT"] {
            swap(&format!("HEADLIGHT_{side}"), &headlight);
            swap(&format!("HEADLIGHT_GLASS_{side}"), "WINDOW_FRONT");
        }
        for side in ["LEFT", "RIGHT", "CENTRE"] {
            swap(&format!("BRAKELIGHT_{side}"), &brakelight);
            swap(&format!("BRAKELIGHT_GLASS_{side}"), &brakelight_glass);
        }
        for window in WINDOWS {
            swap(window, "WINDOW_FRONT");
        }
        Self { map, hidden: DECAL_PLACEHOLDERS.iter().map(|n| bstring_hash(n)).collect() }
    }

    /// The texture to draw for `hash` (itself when nothing replaces it).
    pub fn resolve(&self, hash: u32) -> u32 {
        self.map.get(&hash).copied().unwrap_or(hash)
    }

    /// Like [`Self::resolve`] for a placement; left brakes show the mirrored caliper.
    pub fn resolve_for(&self, hash: u32, left_brake: bool) -> u32 {
        if left_brake && hash == bstring_hash(BRAKE) { bstring_hash(BRAKE_LEFT) } else { self.resolve(hash) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_swaps() {
        let s = TextureSwaps::for_car("BMWM3GTR");
        assert_eq!(s.resolve(bstring_hash("HEADLIGHT_LEFT")), bstring_hash("BMWM3GTR_KIT00_HEADLIGHT_ON"));
        assert_eq!(s.resolve(bstring_hash("WINDOW_REAR")), bstring_hash("WINDOW_FRONT"));
        assert_eq!(s.resolve(bstring_hash("BMWM3GTR_TIRE")), bstring_hash("BMWM3GTR_TIRE"));
        assert_eq!(s.resolve_for(bstring_hash(BRAKE), true), bstring_hash(BRAKE_LEFT));
        assert_eq!(s.resolve_for(bstring_hash(BRAKE), false), bstring_hash(BRAKE));
        assert!(s.hidden.contains(&bstring_hash("LEFT_DOOR_DECAL")));
        assert_eq!(DECAL_PLACEHOLDERS.len(), 8 + 26);
    }
}
