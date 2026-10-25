//! The FEng roots of each HUD element, for the Original theme's adapters (`ui/theme/original/hud_slots.rs`).
//!
//! Each list is the list [`super::elements::Layout::update`] uses for the same element, so an adapter emits the
//! objects the HUD shows for it, and no others. Every hash is a name hash (`Runtime::find`), as in `elements.rs`
//! and `bind.rs`. The grouping decisions (the pursuit board, the gear inside the tachometer, the minimap's blips, the
//! bounty row inside the board) are recorded in `docs/specs/ui-themes.md` section 8 and `docs/specs/ui-themes-slots.md`.

use std::sync::{Arc, OnceLock};

use blackbox_feng::fe_hash_upper;

use super::bind::GEAR;
use super::blip_styles;
use super::elements::{
    BOUNTY_ROW, COOLDOWN, NITROUS, PURSUIT_BOARD, PURSUIT_COPS, SPEEDOMETER, TACHOMETER, TURBO_GROUP,
};
use super::minimap::{ARROW, BACKING, GROUP as MINIMAP_GROUP, PIECES as MINIMAP_PIECES};
use super::race;
use super::speedbreaker;

/// The name hashes one HUD element's adapter takes: the roots of its subtrees, and the subtrees that belong to
/// another element and so are left out. A node is in the element when its nearest root-or-excluded ancestor (itself
/// included) is a root.
///
/// The lists are shared (`Arc`): each element builds its roots once, and a clone is two reference counts. The adapters
/// compare the lists by identity to find the node ownership they worked out for the package (`ui/theme/original`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Roots {
    pub roots: Arc<[u32]>,
    pub excluded: Arc<[u32]>,
}

impl Roots {
    fn new(roots: impl IntoIterator<Item = u32>, excluded: impl IntoIterator<Item = u32>) -> Self {
        Self { roots: roots.into_iter().collect(), excluded: excluded.into_iter().collect() }
    }
}

/// The roots of one element, built on the first call and shared after it.
fn shared(cell: &'static OnceLock<Roots>, build: impl FnOnce() -> Roots) -> Roots {
    cell.get_or_init(build).clone()
}

fn name(text: &str) -> u32 {
    fe_hash_upper(text)
}

/// The speedometer group: the digits, the units and the group's own frame.
pub(crate) fn speedometer() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new(SPEEDOMETER, []))
}

/// The tachometer group: the needle, the face, the redline, the shift light and the fill. The gear digit sits inside
/// the group, so it is left out here and belongs to [`gear`].
pub(crate) fn tachometer() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new(TACHOMETER, [name(GEAR)]))
}

/// The gear digit (`3rdPersonGear`), on its own.
pub(crate) fn gear() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new([name(GEAR)], []))
}

/// The vehicle heat group: the flame and the multiplier (`Heat_Meter_Group`), and the group of the two heat arcs that ring
/// the minimap.
const HEAT_GROUP: u32 = 0xC46A_80A9;
const HEAT_ARCS_GROUP: u32 = 0xB0F6_3386;

/// The minimap: the track group (the four tile pieces, and the blips, which the game draws inside it), the backing
/// disc, the player's arrow and the heat group and arcs around it. The blips are left out here and belong to [`blips`].
pub(crate) fn minimap() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || {
        Roots::new(
            [name(MINIMAP_GROUP), name(BACKING), name(ARROW), HEAT_GROUP, HEAT_ARCS_GROUP],
            blip_icons().iter().copied(),
        )
    })
}

/// The four tile pieces of the minimap on their own, for a theme that draws the rest of the minimap itself.
pub(crate) fn minimap_tiles() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new(MINIMAP_PIECES.map(name), []))
}

/// The player's arrow on its own.
pub(crate) fn minimap_arrow() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new([name(ARROW)], []))
}

/// The police detector: its group (the green laser, the direction arrow and the two LED rows), and the backing it stands on
/// in either layout (on its own, or in the rear-view mirror). The cool-down bar is not in this group.
pub(crate) fn police_radar() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new([0x0627_43F5, 0x839E_7D77, 0x9EE0_6631], []))
}

/// The map blips: the pool objects of every style the HUD registers.
pub(crate) fn blips() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new(blip_icons().iter().copied(), []))
}

/// The signed busted/evade meter and its cool-down bar, with the pursuit board and the cops count. The board is the
/// chase's display (timer, cops, meter), so it belongs to the same slot as the meter. The bounty row sits inside the
/// board in the package **[verified]** (`HUD_SingleRace.fng`: `0x8674e6d4` under `PursuitBoardGroup`), but it is its
/// own slot ([`bounty_row`]), so it is left out here.
pub(crate) fn pursuit_meter() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new(PURSUIT_BOARD.into_iter().chain(PURSUIT_COPS).chain(COOLDOWN), BOUNTY_ROW))
}

/// The race readout: the race information group (the place, the clock and its icon, the progress and the held lap group)
/// and the roots the layout shows with it, all inside that group.
pub(crate) fn race_status() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new(race::root_hashes(), []))
}

/// The race leaderboard: `LeaderBoardGroup` and its rows.
pub(crate) fn leaderboard() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new([super::leaderboard::GROUP], []))
}

/// The career bounty row, on its own. It sits inside the pursuit board, which [`pursuit_meter`] leaves it out of.
pub(crate) fn bounty_row() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new(BOUNTY_ROW, []))
}

/// The lines over the road: the generic message group (`Hud_GenericMessage`, a checkpoint, a lap, the race-over line)
/// and the 3-2-1-GO group. Both sit in the radar's group in the package, which no slot takes.
pub(crate) fn banner() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new([race::message_hash(), race::count_hash()], []))
}

/// The nitrous gauge: its group and its bar.
pub(crate) fn nitrous() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new(NITROUS, []))
}

/// The turbo gauge: its group, with the needle inside it.
pub(crate) fn turbo() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || Roots::new([name(TURBO_GROUP)], []))
}

/// The speed breaker meter: its icon, its group, its mask and the linear group.
pub(crate) fn speedbreaker() -> Roots {
    static ROOTS: OnceLock<Roots> = OnceLock::new();
    shared(&ROOTS, || {
        Roots::new(
            [name(speedbreaker::ICON), name(speedbreaker::GROUP), name(speedbreaker::MASK), speedbreaker::LINEAR_GROUP],
            [],
        )
    })
}

/// The name hashes of the map blips' pool objects: the built-in styles, which `HudBinding::new` registers, and the
/// map icon styles, which the HUD plugin adds. The same list `BlipBinding::icon_hashes` gives for the live binding.
/// Hashed once.
pub(crate) fn blip_icons() -> &'static [u32] {
    static ICONS: OnceLock<Vec<u32>> = OnceLock::new();
    ICONS.get_or_init(|| {
        blip_styles::builtin()
            .into_iter()
            .chain(blip_styles::map_icon_styles())
            .flat_map(|style| style.icons())
            .map(|icon| name(&icon))
            .collect()
    })
}
