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
    "DUMMY_DECAL7",
    "DUMMY_DECAL8",
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
    broken_heads: u32,
    lit_lamps: HashSet<u32>,
    /// Placeholder hash → texture hash to draw instead.
    pub map: HashMap<u32, u32>,
    /// Placeholders whose groups are not drawn.
    pub hidden: HashSet<u32>,
}

impl TextureSwaps {
    /// True for an enabled lamp with authored ON art.
    pub fn lamp_is_on(&self, placeholder: u32) -> bool {
        self.lit_lamps.contains(&placeholder)
    }
    /// Lamp body slots whose ON state is a red lamp: the brake lamps and a mod car's tail lamps.
    pub fn is_brake_lamp(placeholder: u32) -> bool {
        [
            "BRAKELIGHT_LEFT",
            "BRAKELIGHT_RIGHT",
            "BRAKELIGHT_CENTRE",
            "TAILLIGHT_LEFT",
            "TAILLIGHT_RIGHT",
            "TAILLIGHT_CENTRE",
        ]
        .map(bstring_hash)
        .contains(&placeholder)
    }
    /// Suppress each broken headlight independently when selecting lamp textures.
    pub fn with_broken_head_lights(mut self, broken: u32) -> Self {
        self.broken_heads = broken & 3;
        self
    }
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
            // A mod car's tail lamps burn with the headlights and share the housing, so the damage, of the brake lamps.
            ("TAILLIGHT", headlights, &["LEFT", "RIGHT", "CENTRE"][..]),
        ] {
            for side in sides {
                let brake_bit = match (kind.starts_with("BRAKELIGHT") || kind == "TAILLIGHT", *side) {
                    (true, "LEFT") => 8,
                    (true, "RIGHT") => 16,
                    (true, "CENTRE") => 32,
                    _ => 0,
                };
                let enabled = enabled && self.broken_brakes & brake_bit == 0;
                let head_bit = match (kind == "HEADLIGHT", *side) {
                    (true, "LEFT") => 1,
                    (true, "RIGHT") => 2,
                    _ => 0,
                };
                let enabled = enabled && self.broken_heads & head_bit == 0;
                let placeholder = bstring_hash(&format!("{kind}_{side}"));
                let Some(states) = self.lamps.get(&placeholder) else { continue };
                self.lit_lamps.remove(&placeholder);
                if enabled
                    && kind != "HEADLIGHT_GLASS"
                    && kind != "BRAKELIGHT_GLASS"
                    && textures.contains_key(&states[1])
                {
                    self.lit_lamps.insert(placeholder);
                }
                let wanted = states[usize::from(enabled)];
                if textures.contains_key(&wanted) {
                    self.map.insert(placeholder, wanted);
                }
            }
        }
        self
    }

    /// Select the reverse lamp textures of a mod car: `<car>_KIT00_REVERSELIGHT_ON` while the gear is reverse, `_OFF`
    /// otherwise. The game's own cars have no such placeholder, so for them nothing changes. A missing state texture keeps
    /// the current replacement, as for the other lamps; the reverse lamps have no damage bit.
    pub fn with_reverse_state(mut self, reversing: bool, textures: &HashMap<u32, blackbox_tpk::Texture>) -> Self {
        for side in ["LEFT", "RIGHT", "CENTRE"] {
            let placeholder = bstring_hash(&format!("REVERSELIGHT_{side}"));
            let Some(states) = self.lamps.get(&placeholder) else { continue };
            self.lit_lamps.remove(&placeholder);
            if reversing && textures.contains_key(&states[1]) {
                self.lit_lamps.insert(placeholder);
            }
            let wanted = states[usize::from(reversing)];
            if textures.contains_key(&wanted) {
                self.map.insert(placeholder, wanted);
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
        let taillight = format!("{car}_KIT00_TAILLIGHT_ON");
        let reverselight = format!("{car}_KIT00_REVERSELIGHT_OFF");
        for side in ["LEFT", "RIGHT", "CENTRE"] {
            swap(&format!("BRAKELIGHT_{side}"), &brakelight);
            swap(&format!("BRAKELIGHT_GLASS_{side}"), &brakelight_glass);
            swap(&format!("TAILLIGHT_{side}"), &taillight);
            swap(&format!("REVERSELIGHT_{side}"), &reverselight);
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
            ("TAILLIGHT", &["LEFT", "RIGHT", "CENTRE"][..]),
            ("REVERSELIGHT", &["LEFT", "RIGHT", "CENTRE"][..]),
        ] {
            let states = ["OFF", "ON"].map(|state| bstring_hash(&format!("{car}_KIT00_{kind}_{state}")));
            for side in sides {
                lamps.insert(bstring_hash(&format!("{kind}_{side}")), states);
            }
        }
        Self {
            map,
            lamps,
            hidden: DECAL_PLACEHOLDERS.iter().map(|n| bstring_hash(n)).collect(),
            broken_brakes: 0,
            broken_heads: 0,
            lit_lamps: HashSet::new(),
        }
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

    fn texture() -> blackbox_tpk::Texture {
        blackbox_tpk::Texture {
            name: String::new(),
            name_hash: 0,
            width: 1,
            height: 1,
            mip_levels: 1,
            format: blackbox_tpk::PixelFormat::Argb8888,
            compression_type: 32,
            alpha_usage: blackbox_tpk::AlphaUsage::None,
            alpha_sorting: false,
            alpha_blend: 0,
            flags: 0,
            data: vec![0; 4],
            palette: Vec::new(),
        }
    }

    #[test]
    fn experimental_lamp_states_follow_independent_damage_and_missing_art() {
        let textures = ["HEADLIGHT", "BRAKELIGHT"]
            .into_iter()
            .flat_map(|kind| {
                ["ON", "OFF"].map(move |state| (bstring_hash(&format!("CAR_KIT00_{kind}_{state}")), texture()))
            })
            .collect::<HashMap<_, _>>();
        let base = TextureSwaps::for_car("CAR");
        let state =
            base.clone().with_broken_head_lights(1).with_broken_brake_lights(16).with_lamp_state(true, true, &textures);
        for (name, expected) in [
            ("HEADLIGHT_LEFT", false),
            ("HEADLIGHT_RIGHT", true),
            ("BRAKELIGHT_LEFT", true),
            ("BRAKELIGHT_RIGHT", false),
            ("BRAKELIGHT_CENTRE", true),
        ] {
            assert_eq!(state.lamp_is_on(bstring_hash(name)), expected, "{name}");
        }
        assert_eq!(state.resolve(bstring_hash("HEADLIGHT_LEFT")), bstring_hash("CAR_KIT00_HEADLIGHT_OFF"));
        let repaired = base.clone().with_lamp_state(true, true, &textures);
        assert!(repaired.lamp_is_on(bstring_hash("HEADLIGHT_LEFT")));
        let mut missing = textures;
        missing.remove(&bstring_hash("CAR_KIT00_HEADLIGHT_OFF"));
        let fallback = base.with_lamp_state(true, true, &missing);
        assert!(fallback.lamp_is_on(bstring_hash("HEADLIGHT_LEFT")));
        let broken = fallback.with_broken_head_lights(1).with_lamp_state(true, true, &missing);
        assert!(!broken.lamp_is_on(bstring_hash("HEADLIGHT_LEFT")));
    }

    fn state_textures() -> HashMap<u32, blackbox_tpk::Texture> {
        ["HEADLIGHT", "BRAKELIGHT", "TAILLIGHT", "REVERSELIGHT"]
            .into_iter()
            .flat_map(|kind| {
                ["ON", "OFF"].map(move |state| (bstring_hash(&format!("CAR_KIT00_{kind}_{state}")), texture()))
            })
            .collect()
    }

    #[test]
    fn a_mod_cars_tail_lamps_follow_the_headlights_and_share_the_brake_damage() {
        let textures = state_textures();
        let base = TextureSwaps::for_car("CAR");
        let dark = base.clone().with_lamp_state(false, false, &textures);
        let night = base.clone().with_lamp_state(true, false, &textures);
        for side in ["LEFT", "RIGHT", "CENTRE"] {
            let placeholder = bstring_hash(&format!("TAILLIGHT_{side}"));
            assert!(!dark.lamp_is_on(placeholder) && night.lamp_is_on(placeholder), "{side}");
            assert_eq!(dark.resolve(placeholder), bstring_hash("CAR_KIT00_TAILLIGHT_OFF"));
            assert_eq!(night.resolve(placeholder), bstring_hash("CAR_KIT00_TAILLIGHT_ON"));
        }
        let broken = base.with_broken_brake_lights(8).with_lamp_state(true, false, &textures);
        assert!(!broken.lamp_is_on(bstring_hash("TAILLIGHT_LEFT")));
        assert!(broken.lamp_is_on(bstring_hash("TAILLIGHT_RIGHT")));
        assert!(TextureSwaps::is_brake_lamp(bstring_hash("TAILLIGHT_LEFT")), "the tail lamps light red");
    }

    #[test]
    fn a_mod_cars_reverse_lamps_follow_the_gear_alone() {
        let textures = state_textures();
        let base = TextureSwaps::for_car("CAR");
        let forward = base.clone().with_lamp_state(true, true, &textures).with_reverse_state(false, &textures);
        let reverse = base.clone().with_lamp_state(false, false, &textures).with_reverse_state(true, &textures);
        for side in ["LEFT", "RIGHT", "CENTRE"] {
            let placeholder = bstring_hash(&format!("REVERSELIGHT_{side}"));
            assert!(!forward.lamp_is_on(placeholder) && reverse.lamp_is_on(placeholder), "{side}");
            assert_eq!(forward.resolve(placeholder), bstring_hash("CAR_KIT00_REVERSELIGHT_OFF"));
            assert_eq!(reverse.resolve(placeholder), bstring_hash("CAR_KIT00_REVERSELIGHT_ON"));
        }
        assert!(!TextureSwaps::is_brake_lamp(bstring_hash("REVERSELIGHT_CENTRE")), "the reverse lamps light white");
        let mut missing = textures;
        missing.remove(&bstring_hash("CAR_KIT00_REVERSELIGHT_ON"));
        let fallback = base.with_reverse_state(true, &missing);
        assert!(!fallback.lamp_is_on(bstring_hash("REVERSELIGHT_CENTRE")), "no lit art, no emission");
        assert_eq!(fallback.resolve(bstring_hash("REVERSELIGHT_CENTRE")), bstring_hash("CAR_KIT00_REVERSELIGHT_OFF"));
    }

    #[test]
    fn the_game_s_own_cars_are_not_changed_by_the_new_lamp_states() {
        let textures: HashMap<u32, blackbox_tpk::Texture> = HashMap::new();
        let stock = TextureSwaps::for_car("BMWM3GTR")
            .with_lamp_state(true, true, &textures)
            .with_reverse_state(true, &textures);
        assert!(
            !stock.lamp_is_on(bstring_hash("REVERSELIGHT_CENTRE")) && !stock.lamp_is_on(bstring_hash("TAILLIGHT_LEFT"))
        );
    }

    #[test]
    fn stock_swaps() {
        let s = TextureSwaps::for_car("BMWM3GTR");
        assert_eq!(s.resolve(bstring_hash("HEADLIGHT_LEFT")), bstring_hash("BMWM3GTR_KIT00_HEADLIGHT_ON"));
        assert_eq!(s.resolve(bstring_hash("WINDOW_REAR")), bstring_hash("WINDOW_FRONT"));
        assert_eq!(s.resolve(bstring_hash("BMWM3GTR_TIRE")), bstring_hash("BMWM3GTR_TIRE"));
        assert_eq!(s.resolve_for(bstring_hash(BRAKE), true), bstring_hash(BRAKE_LEFT));
        assert_eq!(s.resolve_for(bstring_hash(BRAKE), false), bstring_hash(BRAKE));
        assert!(s.hidden.contains(&bstring_hash("LEFT_DOOR_DECAL")));
        assert_eq!(DECAL_PLACEHOLDERS.len(), 10 + 26);
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
