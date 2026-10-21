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

/// The cracked-window texture of the global pack (`GLOBAL/GLOBALB.BUN`, `WINDOW_DAMAGE0`).
pub const WINDOW_DAMAGE: &str = "WINDOW_DAMAGE0";

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
    lamps: HashMap<u32, [u32; 2]>,
    broken_brakes: u32,
    /// Placeholder hash → texture hash to draw instead.
    pub map: HashMap<u32, u32>,
    /// Placeholders whose groups are not drawn.
    pub hidden: HashSet<u32>,
}

impl TextureSwaps {
    /// Original per-side brake damage; head surface substitutions stay independent.
    pub fn with_broken_brake_lights(mut self, broken: u32) -> Self {
        self.broken_brakes = broken & 56;
        self
    }

    /// Available state textures are loaded once, including both diffuse and brake glass.
    pub fn lamp_variants(&self) -> impl Iterator<Item = u32> + '_ {
        self.lamps.values().flat_map(|states| *states)
    }

    /// Select existing authored textures. Absent variants retain the current replacement.
    pub fn with_lamp_state(
        mut self,
        headlights: bool,
        braking: bool,
        textures: &HashMap<u32, blackbox_tpk::Texture>,
    ) -> Self {
        for (kind, enabled, sides) in [
            ("HEADLIGHT", headlights, &["LEFT", "RIGHT"][..]),
            ("HEADLIGHT_GLASS", true, &["LEFT", "RIGHT"][..]),
            ("BRAKELIGHT", braking, &["LEFT", "RIGHT", "CENTRE"][..]),
            ("BRAKELIGHT_GLASS", braking, &["LEFT", "RIGHT", "CENTRE"][..]),
        ] {
            for side in sides {
                let brake_bit = match (kind.starts_with("BRAKELIGHT"), *side) {
                    (true, "LEFT") => 8,
                    (true, "RIGHT") => 16,
                    (true, "CENTRE") => 32,
                    _ => 0,
                };
                let enabled = enabled && self.broken_brakes & brake_bit == 0;
                let placeholder = bstring_hash(&format!("{kind}_{side}"));
                let Some(states) = self.lamps.get(&placeholder) else { continue };
                let wanted = states[usize::from(enabled)];
                if textures.contains_key(&wanted) {
                    self.map.insert(placeholder, wanted);
                }
            }
        }
        self
    }

    /// Stock swaps for a car type. Traffic (usage 2) shares the global tire atlas;
    /// the original placeholder is based on the type's base model name, not its folder.
    pub fn for_car_type(base_model_name: &str, usage_type: i32) -> Self {
        let mut swaps = Self::for_car(base_model_name);
        if usage_type != 2 {
            return swaps;
        }
        swaps.map.insert(bstring_hash(&format!("{base_model_name}_TIRE")), bstring_hash("TRAFFIC_TIRE"));
        swaps
    }

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
        let mut lamps = HashMap::new();
        for (kind, sides) in [
            ("HEADLIGHT", &["LEFT", "RIGHT"][..]),
            ("HEADLIGHT_GLASS", &["LEFT", "RIGHT"][..]),
            ("BRAKELIGHT", &["LEFT", "RIGHT", "CENTRE"][..]),
            ("BRAKELIGHT_GLASS", &["LEFT", "RIGHT", "CENTRE"][..]),
        ] {
            let states = ["OFF", "ON"].map(|state| bstring_hash(&format!("{car}_KIT00_{kind}_{state}")));
            for side in sides {
                lamps.insert(bstring_hash(&format!("{kind}_{side}")), states);
            }
        }
        Self { map, lamps, hidden: DECAL_PLACEHOLDERS.iter().map(|n| bstring_hash(n)).collect(), broken_brakes: 0 }
    }

    /// Show the cracked-window texture on these texture hashes (window placeholders or window solids' diffuse
    /// textures).
    pub fn with_window_hashes(mut self, hashes: &[u32]) -> Self {
        for &hash in hashes {
            self.map.insert(hash, bstring_hash(WINDOW_DAMAGE));
        }
        self
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

    #[test]
    fn shared_tire_atlas_is_scoped_to_traffic_usage() {
        let original = bstring_hash("TRAILERLOG_TIRE");
        for usage in [0, 1, 3, 4] {
            assert_eq!(TextureSwaps::for_car_type("TRAILERLOG", usage).resolve(original), original);
        }
        let swaps = TextureSwaps::for_car_type("TRAILERLOG", 2);
        assert_eq!(swaps.resolve(original), bstring_hash("TRAFFIC_TIRE"));
        assert_eq!(swaps.resolve(bstring_hash("OTHER_TIRE")), bstring_hash("OTHER_TIRE"));
        assert_eq!(swaps.resolve(bstring_hash("WINDOW_REAR")), bstring_hash("WINDOW_FRONT"));
        assert!(swaps.hidden.contains(&bstring_hash("LEFT_DOOR_DECAL")));
    }
}
