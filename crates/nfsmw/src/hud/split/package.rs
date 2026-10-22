//! The four split-screen HUD packages and how a race picks them (`docs/specs/split-screen-hud.md` section 1).

use blackbox_feng::Package;
use game_install::GameDir;

use crate::ui::Catalog;

/// The bun that holds the four packages. The original loads it only in a two-player Quick Race.
pub const SPLIT_FILE: &str = "GLOBAL/InGameSplitScreen.bun";

/// Which player's HUD: player 1 has the top half of the screen, player 2 the bottom half.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seat {
    First,
    Second,
}

impl Seat {
    pub const BOTH: [Seat; 2] = [Seat::First, Seat::Second];

    /// The player's number as the scene counts them: 0 the first, 1 the second (`Scene::split_hud_state`).
    pub fn index(self) -> usize {
        match self {
            Seat::First => 0,
            Seat::Second => 1,
        }
    }
}

/// The kind of race the HUDs are for: a drag race has its own pair of packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Race,
    Drag,
}

impl Kind {
    pub const BOTH: [Kind; 2] = [Kind::Race, Kind::Drag];

    pub fn of(drag: bool) -> Self {
        match drag {
            true => Kind::Drag,
            false => Kind::Race,
        }
    }

    /// The package of a player's HUD.
    pub fn package(self, seat: Seat) -> &'static str {
        match (self, seat) {
            (Kind::Race, Seat::First) => "HUD_Player1.fng",
            (Kind::Race, Seat::Second) => "HUD_Player2.fng",
            (Kind::Drag, Seat::First) => "HUD_Drag_Player1.fng",
            (Kind::Drag, Seat::Second) => "HUD_Drag_Player2.fng",
        }
    }

    #[allow(dead_code)] // the number the scene hook is keyed by in the original; documented, not needed here
    /// The layout number the game maps to the package (docs/provenance/split-screen.md): 3 and 4 are the race HUDs of players 1 and 2, 5
    /// and 6 the drag ones **[exe]**.
    pub fn layout_number(self, seat: Seat) -> u32 {
        match (self, seat) {
            (Kind::Race, Seat::First) => 3,
            (Kind::Race, Seat::Second) => 4,
            (Kind::Drag, Seat::First) => 5,
            (Kind::Drag, Seat::Second) => 6,
        }
    }

    #[allow(dead_code)] // the pack is loaded with the shared textures (`ui/assets.rs`)
    /// The texture pack of the pair, under `GLOBAL/`. The decomp's constants for the two names are swapped; the packs'
    /// contents settle it (`2PlayerDrag_*` is in `HUDTEXDRAGSPLIT.BIN`) **[verified]**.
    pub fn texture_pack(self) -> &'static str {
        match self {
            Kind::Race => "GLOBAL/HUDTEXSPLIT.BIN",
            Kind::Drag => "GLOBAL/HUDTEXDRAGSPLIT.BIN",
        }
    }
}

/// The two packages of a kind, if the install has both.
pub fn load(dir: &GameDir, kind: Kind) -> Option<[Package; 2]> {
    let catalog = Catalog::load(dir, &[SPLIT_FILE]);
    let find = |seat| catalog.find(kind.package(seat)).cloned();
    Some([find(Seat::First)?, find(Seat::Second)?])
}
