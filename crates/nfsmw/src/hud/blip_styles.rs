//! The blips the original game draws, as [`BlipStyle`]s: the cop, the racer, the checkpoint and the helicopter, and
//! the race events and interiors. Their icons are the package's objects (`docs/formats/minimap.md`,
//! `docs/specs/hud-minimap.md` section 7).

use super::blip_style::{BlipLook, BlipStyle, MapBlip, Visibility};
use super::cop_blip;

pub const COP: &str = "cop";
pub const RACER: &str = "racer";
pub const CHECKPOINT: &str = "checkpoint";
pub const HELICOPTER: &str = "helicopter";
pub const RACE_SPRINT: &str = "race_sprint";
pub const RACE_DRAG: &str = "race_drag";
pub const RACE_CIRCUIT: &str = "race_circuit";
pub const RACE_KNOCKOUT: &str = "race_knockout";
pub const RACE_TOLLBOOTH: &str = "race_tollbooth";
pub const RACE_SPEEDTRAP: &str = "race_speedtrap";
pub const AUTO_SHOP: &str = "auto_shop";
pub const CAR_LOT: &str = "car_lot";
pub const SAFE_HOUSE: &str = "safe_house";

/// The objects of one race type's pool: `MMICON_<TYPE>_0..7`.
const RACE_POOL: usize = 8;
/// The auto shops' pool: `SHOP_0..7`, one per shop zone.
const AUTO_SHOP_POOL: usize = 8;
/// The safe houses' pool: `SAFEHOUSEINDICATOR_0..3`, one per safe house.
const SAFE_HOUSE_POOL: usize = 4;
/// The car lots' pool: `CAR_LOT_0..3`, one per car lot.
const CAR_LOT_POOL: usize = 4;

/// The tints of the interiors where the package object is plain white. The shop's yellow and the car lot's blue are
/// the game's own colours (the roadblock's yellow, the cop's blue). The safe house's green is from the first
/// screenshot's legend, not from the executable (see `docs/specs/hud-minimap.md` section 7).
pub const AUTO_SHOP_TINT: [u8; 4] = [0xff, 0xed, 0x00, 0xff];
pub const CAR_LOT_TINT: [u8; 4] = [0x00, 0x00, 0xff, 0xff];
pub const SAFE_HOUSE_TINT: [u8; 4] = [0x00, 0xff, 0x00, 0xff];

/// A police car: the car arrow, turned to its heading, with a light bar that cycles blue, grey and red while `active`
/// (a pursuit). A pursuing car is always on the map. The others show only inside the minimap's view.
pub struct Cop;

impl BlipStyle for Cop {
    fn name(&self) -> &'static str {
        COP
    }

    fn icons(&self) -> Vec<String> {
        (0..cop_blip::ICON_COUNT).map(cop_blip::icon_name).collect()
    }

    fn visibility(&self, blip: &MapBlip) -> Visibility {
        if blip.active {
            return Visibility::Always;
        }
        Visibility::InZone
    }

    fn look(&self, blip: &MapBlip, tick: u32) -> BlipLook {
        BlipLook { colour: Some(cop_blip::colour(blip.active, cop_blip::step_at(tick))), ..BlipLook::PLAIN }
    }
}

/// A racer in a race: `MMICON_AIRACER_0..7`.
pub struct Racer;

impl BlipStyle for Racer {
    fn name(&self) -> &'static str {
        RACER
    }

    fn icons(&self) -> Vec<String> {
        (0..8).map(|i| format!("MMICON_AIRACER_{i}")).collect()
    }
}

/// The race checkpoint. It shows only while `active` (the condition the race sets).
pub struct Checkpoint;

impl BlipStyle for Checkpoint {
    fn name(&self) -> &'static str {
        CHECKPOINT
    }

    fn icons(&self) -> Vec<String> {
        vec!["MMICON_CHECKPOINT".to_string()]
    }

    fn shows(&self, blip: &MapBlip) -> bool {
        blip.active
    }
}

/// The police helicopter.
pub struct Helicopter;

impl BlipStyle for Helicopter {
    fn name(&self) -> &'static str {
        HELICOPTER
    }

    fn icons(&self) -> Vec<String> {
        vec!["HELICOPTER_ICON_GROUP".to_string()]
    }
}

/// A pool of HUD objects named `<prefix><index>` for `index` in `0..size`: one object per blip shown at once.
/// By default its blips are always eligible, keep the package colour and are all drawn.
pub struct IconPool {
    name: &'static str,
    prefix: &'static str,
    size: usize,
    visibility: Visibility,
    nearest: bool,
    fallback: Option<[u8; 4]>,
}

impl IconPool {
    pub const fn new(name: &'static str, prefix: &'static str, size: usize) -> Self {
        Self { name, prefix, size, visibility: Visibility::Always, nearest: false, fallback: None }
    }

    /// Drawn only while the view covers the blip's zone.
    pub fn in_zone(mut self) -> Self {
        self.visibility = Visibility::InZone;
        self
    }

    /// Only the nearest blip of the style is drawn.
    pub fn nearest_only(mut self) -> Self {
        self.nearest = true;
        self
    }

    /// The tint where the package object is plain white.
    pub fn tinted(mut self, colour: [u8; 4]) -> Self {
        self.fallback = Some(colour);
        self
    }
}

impl BlipStyle for IconPool {
    fn name(&self) -> &'static str {
        self.name
    }

    fn icons(&self) -> Vec<String> {
        (0..self.size).map(|i| format!("{}{i}", self.prefix)).collect()
    }

    fn visibility(&self, _blip: &MapBlip) -> Visibility {
        self.visibility
    }

    fn nearest_only(&self) -> bool {
        self.nearest
    }

    fn fallback_colour(&self) -> Option<[u8; 4]> {
        self.fallback
    }
}

/// The styles of the race events (one per race type that has an icon in the package), the auto shops, the car lots
/// and the safe houses. Blips of one style take that style's objects in order; a blip past the pool is not drawn.
/// The race blips are drawn only while the view covers their zone; the shops and car lots show only the nearest one
/// of their kind; the safe houses always show (`docs/specs/hud-minimap.md` section 7).
pub fn map_icon_styles() -> Vec<Box<dyn BlipStyle>> {
    vec![
        Box::new(IconPool::new(RACE_SPRINT, "MMICON_SPRINT_", RACE_POOL).in_zone()),
        Box::new(IconPool::new(RACE_DRAG, "MMICON_DRAG_", RACE_POOL).in_zone()),
        Box::new(IconPool::new(RACE_CIRCUIT, "MMICON_CIRCUIT_", RACE_POOL).in_zone()),
        Box::new(IconPool::new(RACE_KNOCKOUT, "MMICON_KNOCKOUT_", RACE_POOL).in_zone()),
        Box::new(IconPool::new(RACE_TOLLBOOTH, "MMICON_TOLLBOOTH_", RACE_POOL).in_zone()),
        Box::new(IconPool::new(RACE_SPEEDTRAP, "MMICON_SPEEDTRAP_", RACE_POOL).in_zone()),
        Box::new(IconPool::new(AUTO_SHOP, "SHOP_", AUTO_SHOP_POOL).nearest_only().tinted(AUTO_SHOP_TINT)),
        Box::new(IconPool::new(CAR_LOT, "CAR_LOT_", CAR_LOT_POOL).nearest_only().tinted(CAR_LOT_TINT)),
        Box::new(IconPool::new(SAFE_HOUSE, "SAFEHOUSEINDICATOR_", SAFE_HOUSE_POOL).tinted(SAFE_HOUSE_TINT)),
    ]
}

/// The styles the game uses.
pub fn builtin() -> Vec<Box<dyn BlipStyle>> {
    vec![Box::new(Cop), Box::new(Racer), Box::new(Checkpoint), Box::new(Helicopter)]
}
