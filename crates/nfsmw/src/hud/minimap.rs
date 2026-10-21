//! The minimap of the HUD: the objects of `HUD_SingleRace.fng` that the game moves each frame
//! (`docs/specs/hud-minimap.md` section 4). The maths is `blackbox-minimap`; this finds the objects, hands them the
//! tile textures of the view, scrolls and turns the group of pieces, and turns the arrow.

use std::sync::Mutex;

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};
use blackbox_minimap::{Blip, Calibration, Orientation, View, place_blip, speed_zoom, tile_name};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_career::regions::MapVariant;
use nfsmw_data::minimap::{GRID, MAX_SPEED};

use super::state::{HudState, MapPosition};
use crate::ui::UiAssets;

/// The group of the four pieces, its backing disc and the arrow: what the minimap shows and the layout turns on.
pub const GROUP: &str = "TRACK_MAP";
pub const BACKING: &str = "TRACKMAPTARGETRING";
pub const ARROW: &str = "PLAYERCARINDICATOR";
/// The pieces, top left, top right, bottom left, bottom right (the order of the view's tiles).
pub const PIECES: [&str; 4] = ["TRACK_MAP1", "TRACK_MAP2", "TRACK_MAP3", "TRACK_MAP4"];
/// The game's `MinimapPivotX` and `MinimapPivotY`: zero for the 4:3 HUD (they move with the widescreen layout).
const PIVOT_SHIFT: [f32; 2] = [0.0, 0.0];
/// The width of the whole picture on the HUD, in HUD units (`docs/specs/hud-minimap.md` section 7).
const PICTURE_WIDTH: f32 = 1024.0;

/// The minimap file stem of a variant, upper case: its tile textures are `<stem>_CHOP<n>`.
pub fn variant_stem(variant: MapVariant) -> &'static str {
    variant.minimap_file().trim_end_matches(".BIN")
}

/// The variant the HUD wants: the career stage's in career mode (`None` outside it), the full map otherwise.
pub fn variant_for_stage(stage: Option<u8>) -> MapVariant {
    match stage {
        Some(stage) => MapVariant::of_stage(stage),
        None => MapVariant::Full,
    }
}

/// The variant to show: the wanted one when its tiles are loaded, else the full map (its tiles are always loaded).
pub fn usable_variant(wanted: MapVariant, loaded: impl Fn(MapVariant) -> bool) -> MapVariant {
    if loaded(wanted) {
        return wanted;
    }
    MapVariant::Full
}

/// The minimap objects and what is needed to drive them.
pub struct MinimapBinding {
    calibration: Calibration,
    /// The map file's name, upper case: the tile textures are `<header>_CHOP<n>`.
    header: String,
    group: ObjectRef,
    pieces: [ObjectRef; 4],
    masks: [u32; 4],
    arrow: ObjectRef,
    /// Where the package puts the group, and the mask rectangle of each piece.
    group_position: Vec3,
    mask_windows: [[f32; 4]; 4],
    /// The tile number each piece last showed and the texture key hashed from it. A piece is renamed only when its
    /// tile changes, so a frame on the same tiles formats and hashes nothing.
    tile_keys: Mutex<[Option<(i32, u32)>; 4]>,
}

impl MinimapBinding {
    /// Finds the objects; `None` when the package lacks one (a different HUD).
    pub fn new(rt: &Runtime, package: PackageId, calibration: Calibration, header: &str) -> Option<Self> {
        if !calibration.is_valid() {
            return None;
        }
        let find = |name: &str| rt.find(package, fe_hash_upper(name));
        let group = find(GROUP)?;
        let arrow = find(ARROW)?;
        let mut pieces = [group; 4];
        let mut mask_windows = [[0.0; 4]; 4];
        let mut masks = [0; 4];
        for (i, name) in PIECES.iter().enumerate() {
            pieces[i] = find(name)?;
            masks[i] = rt.package(package)?.objects.get(pieces[i].index)?.multi?.textures[0];
            if masks[i] == 0 {
                return None;
            }
            mask_windows[i] = rt.object(pieces[i])?.data.multi_uv(0);
        }
        Some(Self {
            calibration,
            header: header.to_ascii_uppercase(),
            group,
            pieces,
            masks,
            arrow,
            group_position: rt.position(group)?,
            mask_windows,
            tile_keys: Mutex::new([None; 4]),
        })
    }

    /// The minimap of the open city's full map: see [`MinimapBinding::open_variant`].
    pub fn open_city(rt: &Runtime, package: PackageId, dir: &GameDir, assets: &UiAssets) -> Option<Self> {
        Self::open_variant(rt, package, dir, assets, MapVariant::Full)
    }

    /// The minimap of the open city with the tiles of `variant`: the calibration from the install and those tiles,
    /// which have to be in `assets`. `None` (with a log line) when anything is missing.
    pub fn open_variant(
        rt: &Runtime,
        package: PackageId,
        dir: &GameDir,
        assets: &UiAssets,
        variant: MapVariant,
    ) -> Option<Self> {
        let stem = variant_stem(variant);
        let calibration = match nfsmw_data::minimap::open_city_calibration(dir) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("the minimap is off: {e:#}");
                return None;
            }
        };
        let Some(binding) = Self::new(rt, package, calibration, stem) else {
            log::warn!("the minimap {stem} is off: invalid calibration or incomplete HUD objects");
            return None;
        };
        let complete = binding.resources_available(
            |key| assets.texture(key).is_some(),
            |key| assets.image(key).is_some_and(|image| image.width > 0 && image.height > 0),
        );
        if !complete {
            log::warn!("the minimap {stem} is off: a map tile or readable mask texture is missing");
            return None;
        }
        Some(binding)
    }

    /// Switches the tiles to the map file `stem` (upper case: `MINI_MAP_UNLOCK_1`, or a race's `MINI_MAP_10_2_1`). The
    /// 64 tiles named `<stem>_CHOP<n>` must be in the assets (see [`usable_variant`] for the stage's maps): the pieces
    /// take them on the next [`MinimapBinding::apply`], and the tile keys hashed for the old file are dropped.
    /// Returns whether anything changed.
    pub fn set_stem(&mut self, stem: &str) -> bool {
        if self.header == stem {
            return false;
        }
        self.header = stem.to_ascii_uppercase();
        *self.tile_keys.get_mut().unwrap_or_else(|e| e.into_inner()) = [None; 4];
        true
    }

    /// Resource validation is separate from install discovery, so incomplete hosts can fail closed.
    pub(super) fn resources_available(
        &self,
        tile_exists: impl Fn(u32) -> bool,
        mask_readable: impl Fn(u32) -> bool,
    ) -> bool {
        let all_tiles = (0..GRID.tiles_per_side * GRID.tiles_per_side)
            .all(|n| tile_exists(fe_hash_upper(&tile_name(&self.header, n))));
        if !all_tiles {
            return false;
        }
        self.masks.iter().enumerate().all(|(i, mask)| self.masks[..i].contains(mask) || mask_readable(*mask))
    }
    /// The view for this frame: the player's place, the car's heading and the speed zoom.
    pub fn view(&self, state: &HudState, at: &MapPosition) -> View {
        View::new(GRID, &self.calibration, at.position, at.heading, speed_zoom(state.speed, MAX_SPEED))
    }

    /// Where a world position and heading land on the picture (`docs/specs/hud-minimap.md` section 7).
    pub(super) fn blip(&self, view: &View, orientation: Orientation, position: [f32; 2], heading: [f32; 2]) -> Blip {
        place_blip(view, orientation, self.calibration.to_map(position), heading, false, PICTURE_WIDTH)
    }

    /// Where the picture's centre is on the HUD, before it scrolls.
    pub(super) fn centre(&self) -> [f32; 2] {
        [self.group_position.x, self.group_position.y]
    }

    /// The texture key of tile `number` for piece `slot`, hashed again only when that piece's tile changed.
    fn tile_key(&self, slot: usize, number: i32) -> u32 {
        // Only the HUD system touches it, so the lock is never contended.
        let mut keys = self.tile_keys.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((cached, key)) = keys[slot]
            && cached == number
        {
            return key;
        }
        let key = fe_hash_upper(&tile_name(&self.header, number));
        keys[slot] = Some((number, key));
        key
    }

    /// Pushes the view into the objects. Call before `Runtime::update`.
    pub fn apply(&self, rt: &mut Runtime, state: &HudState, at: &MapPosition) {
        let view = self.view(state, at);
        let shift = view.mask_shift();
        let rectangles = view.piece_rectangles();
        for (i, piece) in self.pieces.iter().enumerate() {
            rt.set_texture(*piece, self.tile_key(i, view.tiles[i]));
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

#[cfg(test)]
#[path = "minimap_variant_tests.rs"]
mod variant_tests;
