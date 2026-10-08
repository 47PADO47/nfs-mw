//! The minimap of the HUD: the objects of `HUD_SingleRace.fng` that the game moves each frame
//! (`docs/specs/hud-minimap.md` section 4). The maths is `blackbox-minimap`; this finds the objects, hands them the
//! tile textures of the view, scrolls and turns the group of pieces, and turns the arrow.

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};
use blackbox_minimap::{Calibration, View, speed_zoom, tile_name};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::minimap::{FULL_MAP, GRID, MAX_SPEED};

use super::state::{HudState, MapPosition};
use crate::ui::UiAssets;

/// The group of the four pieces, its backing disc and the arrow: what the minimap shows and the layout turns on.
pub const GROUP: &str = "TRACK_MAP";
pub const BACKING: &str = "TRACKMAPTARGETRING";
pub const ARROW: &str = "PLAYERCARINDICATOR";
/// The pieces, top left, top right, bottom left, bottom right (the order of the view's tiles).
const PIECES: [&str; 4] = ["TRACK_MAP1", "TRACK_MAP2", "TRACK_MAP3", "TRACK_MAP4"];
/// The game's `MinimapPivotX` and `MinimapPivotY`: zero for the 4:3 HUD (they move with the widescreen layout).
const PIVOT_SHIFT: [f32; 2] = [0.0, 0.0];

/// The minimap objects and what is needed to drive them.
pub struct MinimapBinding {
    calibration: Calibration,
    /// The map file's name, upper case: the tile textures are `<header>_CHOP<n>`.
    header: String,
    group: ObjectRef,
    pieces: [ObjectRef; 4],
    arrow: ObjectRef,
    /// Where the package puts the group, and the mask rectangle of each piece.
    group_position: Vec3,
    mask_windows: [[f32; 4]; 4],
}

impl MinimapBinding {
    /// Finds the objects; `None` when the package lacks one (a different HUD).
    pub fn new(rt: &Runtime, package: PackageId, calibration: Calibration, header: &str) -> Option<Self> {
        let find = |name: &str| rt.find(package, fe_hash_upper(name));
        let group = find(GROUP)?;
        let arrow = find(ARROW)?;
        let mut pieces = [group; 4];
        let mut mask_windows = [[0.0; 4]; 4];
        for (i, name) in PIECES.iter().enumerate() {
            pieces[i] = find(name)?;
            mask_windows[i] = rt.object(pieces[i])?.data.multi_uv(0);
        }
        Some(Self {
            calibration,
            header: header.to_ascii_uppercase(),
            group,
            pieces,
            arrow,
            group_position: rt.position(group)?,
            mask_windows,
        })
    }

    /// The minimap of the open city: the calibration from the install and the full map's tiles, which have to be in
    /// `assets`. `None` (with a log line) when anything is missing.
    pub fn open_city(rt: &Runtime, package: PackageId, dir: &GameDir, assets: &UiAssets) -> Option<Self> {
        let calibration = match nfsmw_data::minimap::open_city_calibration(dir) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("the minimap is off: {e:#}");
                return None;
            }
        };
        if assets.texture(fe_hash_upper(&tile_name(FULL_MAP, 0))).is_none() {
            log::warn!("the minimap is off: the map tiles of {FULL_MAP} are not in the assets");
            return None;
        }
        let binding = Self::new(rt, package, calibration, FULL_MAP);
        if binding.is_none() {
            log::warn!("the minimap is off: the HUD package lacks its objects");
        }
        binding
    }

    /// The view for this frame: the player's place, the car's heading and the speed zoom.
    pub fn view(&self, state: &HudState, at: &MapPosition) -> View {
        View::new(GRID, &self.calibration, at.position, at.heading, speed_zoom(state.speed, MAX_SPEED))
    }

    /// Pushes the view into the objects. Call before `Runtime::update`.
    pub fn apply(&self, rt: &mut Runtime, state: &HudState, at: &MapPosition) {
        let view = self.view(state, at);
        let shift = view.mask_shift();
        let rectangles = view.piece_rectangles();
        for (i, piece) in self.pieces.iter().enumerate() {
            rt.set_texture(*piece, fe_hash_upper(&tile_name(&self.header, view.tiles[i])));
            rt.set_uv(*piece, rectangles[i]);
            let w = self.mask_windows[i];
            rt.set_mask_uv(*piece, [w[0] + shift[0], w[1] + shift[1], w[2] + shift[0], w[3] + shift[1]]);
        }
        let scroll = view.scroll();
        rt.set_position_xy(self.group, self.group_position.x - scroll[0], self.group_position.y - scroll[1]);
        rt.set_pivot_xy(self.group, scroll[0] + PIVOT_SHIFT[0], scroll[1] + PIVOT_SHIFT[1]);
        rt.set_rotation_z(self.group, view.group_rotation(at.orientation).to_radians());
        rt.set_rotation_z(self.arrow, view.arrow_rotation(at.orientation).to_radians());
    }
}
