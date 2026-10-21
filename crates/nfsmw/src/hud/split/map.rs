//! The minimap of a split-screen race: one map, in the first player's package, centred on the first player and fixed
//! north up (`Minimap::Update` sets `MinimapRotateWithPlayer` to 0 in split screen) **[decomp]**. The second player's car
//! is the package's second arrow, `PLAYERCARINDICATOR2`, placed like any blip: scaled onto the rim when it is off the
//! map (`Minimap::UpdatePlayer2Element`) **[decomp]**. The second player's package has no map **[verified]**.

use blackbox_feng::{PackageId, Runtime, fe_hash_upper};
use blackbox_minimap::Orientation;
use game_install::GameDir;
use nfsmw_career::regions::MapVariant;

use super::names::MAP_ARROW_2;
use crate::hud::blip_style::{BlipStyle, MapBlip};
use crate::hud::blips::BlipBinding;
use crate::hud::event_tiles::{EventTiles, install_loader};
use crate::hud::map_source::resolve_stem;
use crate::hud::minimap::MinimapBinding;
use crate::hud::state::{HudState, MapPosition};
use crate::ui::UiAssets;

/// The blip style of the second player's car.
const SECOND_PLAYER: &str = "second_player";

struct SecondPlayer;

impl BlipStyle for SecondPlayer {
    fn name(&self) -> &'static str {
        SECOND_PLAYER
    }

    fn icons(&self) -> Vec<String> {
        vec![MAP_ARROW_2.to_owned()]
    }
}

pub struct SharedMap {
    binding: MinimapBinding,
    blips: BlipBinding,
    tiles: EventTiles,
}

impl SharedMap {
    /// The map of the first player's package; `None` when the package has none or the map's textures are missing.
    pub fn open(rt: &mut Runtime, package: PackageId, dir: &GameDir, assets: &UiAssets) -> Option<Self> {
        let binding = MinimapBinding::open_city(rt, package, dir, assets)?;
        let blips = BlipBinding::new(rt, package, vec![Box::new(SecondPlayer)]);
        Some(Self { binding, blips, tiles: EventTiles::new(install_loader(dir.clone())) })
    }

    /// The name hashes of the second arrow, for the placement (it moves with the map's side).
    pub fn icon_hashes() -> Vec<u32> {
        vec![fe_hash_upper(MAP_ARROW_2)]
    }

    pub fn advance(&mut self, dt: f32) {
        self.blips.advance(dt);
    }

    /// Follows the race's map file: the tiles of its event once they are loaded, else the full map.
    pub fn follow(&mut self, state: &HudState, assets: &UiAssets) {
        self.tiles.update(state.map_source.event(), assets);
        let stem = resolve_stem(&state.map_source, None, |v| v == MapVariant::Full, self.tiles.ready_stem());
        self.binding.set_stem(stem);
    }

    /// Puts the first player's place in the middle and the second player's car on the map. Without a map position for the
    /// first player the map stays as it was; the caller hides it (`Want::map`).
    pub fn apply(&mut self, rt: &mut Runtime, first: &HudState, second: &HudState) {
        let Some(at) = first.minimap else { return };
        let at = MapPosition { orientation: Orientation::North, ..at };
        let mut shown = first.clone();
        shown.minimap = Some(at);
        shown.blips = second.minimap.map(|p| car_blip(&p)).into_iter().collect();
        self.binding.apply(rt, &shown, &at);
        self.blips.apply(rt, &shown, &self.binding);
    }
}

/// The second player's car as a blip.
fn car_blip(at: &MapPosition) -> MapBlip {
    MapBlip { style: SECOND_PLAYER, position: at.position, active: false, heading: Some(at.heading) }
}
