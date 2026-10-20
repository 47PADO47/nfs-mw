//! The FEng roots of each HUD element, for the Original theme's adapters (`ui/theme/original/hud_slots.rs`).
//!
//! Each list is the list [`super::elements::Layout::update`] uses for the same element, so an adapter emits the
//! objects the HUD shows for it, and no others. Every hash is a name hash (`Runtime::find`), as in `elements.rs`
//! and `bind.rs`. The grouping decisions (the pursuit board, the gear inside the tachometer, the minimap's blips) are
//! recorded in `docs/specs/ui-themes.md` section 8.

use blackbox_feng::fe_hash_upper;

use super::bind::GEAR;
use super::blip_styles;
use super::elements::{COOLDOWN, NITROUS, PURSUIT_BOARD, PURSUIT_COPS, SPEEDOMETER, TACHOMETER, TURBO_GROUP};
use super::minimap::{ARROW, BACKING, GROUP as MINIMAP_GROUP, PIECES as MINIMAP_PIECES};
use super::speedbreaker;

/// The name hashes one HUD element's adapter takes: the roots of its subtrees, and the subtrees that belong to
/// another element and so are left out. A node is in the element when its nearest root-or-excluded ancestor (itself
/// included) is a root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Roots {
    pub roots: Vec<u32>,
    pub excluded: Vec<u32>,
}

impl Roots {
    fn new(roots: impl IntoIterator<Item = u32>, excluded: impl IntoIterator<Item = u32>) -> Self {
        Self { roots: roots.into_iter().collect(), excluded: excluded.into_iter().collect() }
    }
}

fn name(text: &str) -> u32 {
    fe_hash_upper(text)
}

/// The speedometer group: the digits, the units and the group's own frame.
pub(crate) fn speedometer() -> Roots {
    Roots::new(SPEEDOMETER, [])
}

/// The tachometer group: the needle, the face, the redline, the shift light and the fill. The gear digit sits inside
/// the group, so it is left out here and belongs to [`gear`].
pub(crate) fn tachometer() -> Roots {
    Roots::new(TACHOMETER, [name(GEAR)])
}

/// The gear digit (`3rdPersonGear`), on its own.
pub(crate) fn gear() -> Roots {
    Roots::new([name(GEAR)], [])
}

/// The vehicle heat group: the flame and the multiplier (`Heat_Meter_Group`), and the group of the two heat arcs that ring
/// the minimap.
const HEAT_GROUP: u32 = 0xC46A_80A9;
const HEAT_ARCS_GROUP: u32 = 0xB0F6_3386;

/// The minimap: the track group (the four tile pieces, and the blips, which the game draws inside it), the backing
/// disc, the player's arrow and the heat group and arcs around it. The blips are left out here and belong to [`blips`].
pub(crate) fn minimap() -> Roots {
    Roots::new([name(MINIMAP_GROUP), name(BACKING), name(ARROW), HEAT_GROUP, HEAT_ARCS_GROUP], blip_icons())
}

/// The four tile pieces of the minimap on their own, for a theme that draws the rest of the minimap itself.
pub(crate) fn minimap_tiles() -> Roots {
    Roots::new(MINIMAP_PIECES.map(name), [])
}

/// The player's arrow on its own.
pub(crate) fn minimap_arrow() -> Roots {
    Roots::new([name(ARROW)], [])
}

/// The police detector: its group (the green laser, the direction arrow and the two LED rows), and the backing it stands on
/// in either layout (on its own, or in the rear-view mirror). The cool-down bar is not in this group.
pub(crate) fn police_radar() -> Roots {
    Roots::new([0x0627_43F5, 0x839E_7D77, 0x9EE0_6631], [])
}

/// The map blips: the pool objects of every style the HUD registers.
pub(crate) fn blips() -> Roots {
    Roots::new(blip_icons(), [])
}

/// The signed busted/evade meter and its cool-down bar, with the pursuit board and the cops count. The board is the
/// chase's display (timer, cops, meter), so it belongs to the same slot as the meter.
pub(crate) fn pursuit_meter() -> Roots {
    Roots::new(PURSUIT_BOARD.into_iter().chain(PURSUIT_COPS).chain(COOLDOWN), [])
}

/// The nitrous gauge: its group and its bar.
pub(crate) fn nitrous() -> Roots {
    Roots::new(NITROUS, [])
}

/// The turbo gauge: its group, with the needle inside it.
pub(crate) fn turbo() -> Roots {
    Roots::new([name(TURBO_GROUP)], [])
}

/// The speed breaker meter: its icon, its group, its mask and the linear group.
pub(crate) fn speedbreaker() -> Roots {
    Roots::new(
        [name(speedbreaker::ICON), name(speedbreaker::GROUP), name(speedbreaker::MASK), speedbreaker::LINEAR_GROUP],
        [],
    )
}

/// The name hashes of the map blips' pool objects: the built-in styles, which `HudBinding::new` registers, and the
/// map icon styles, which the HUD plugin adds. The same list `BlipBinding::icon_hashes` gives for the live binding.
pub(crate) fn blip_icons() -> Vec<u32> {
    blip_styles::builtin()
        .into_iter()
        .chain(blip_styles::map_icon_styles())
        .flat_map(|style| style.icons())
        .map(|icon| name(&icon))
        .collect()
}
