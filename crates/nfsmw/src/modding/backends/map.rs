//! `bb.map`: blips a mod places on the minimap and the world map.
//!
//! A mod names a blip style the game has (the cop, the helicopter, the race, shop, car lot and safe house icons). The
//! blips are kept here, and [`minimap_blips`] gives them to the scene each frame, which merges them into its own
//! (`scenes/world/hud.rs`). The world map reads the same list, so a mod blip shows there too.

use std::rc::Rc;

use blackbox_modding::abi::BB_ERR_NOT_FOUND;
use blackbox_modding::host::{BackendResult, BlipSpec, MapBackend};

use super::bridge::Bridge;
use crate::hud::{
    AUTO_SHOP, CAR_LOT, CHECKPOINT, COP, HELICOPTER, MapBlip, RACE_CIRCUIT, RACE_DRAG, RACE_KNOCKOUT, RACE_SPEEDTRAP,
    RACE_SPRINT, RACE_TOLLBOOTH, RACER, SAFE_HOUSE,
};

/// The styles a mod may use: the game's blip styles, as `hud` re-exports them.
const STYLES: [&str; 13] = [
    COP,
    HELICOPTER,
    RACE_SPRINT,
    RACE_DRAG,
    RACE_CIRCUIT,
    RACE_KNOCKOUT,
    RACE_TOLLBOOTH,
    RACE_SPEEDTRAP,
    AUTO_SHOP,
    CAR_LOT,
    SAFE_HOUSE,
    RACER,
    CHECKPOINT,
];

/// The `bb.map` port of the game: one per mod table, sharing the bridge.
pub struct MapPort(pub(super) Rc<Bridge>);

/// A blip a mod placed. Its position is `x` forward and `y` left, in metres, as the minimap has it.
#[derive(Debug, Clone, PartialEq)]
pub struct ModBlip {
    pub handle: u64,
    pub owner: String,
    pub style: &'static str,
    pub position: [f32; 2],
    pub active: bool,
}

impl ModBlip {
    fn new(handle: u64, owner: &str, style: &'static str, spec: &BlipSpec<'_>) -> Self {
        Self {
            handle,
            owner: owner.to_owned(),
            style,
            position: [spec.position[0], spec.position[1]],
            active: spec.active,
        }
    }

    /// The blip as the minimap draws it. `bb.map` has no heading, so the icon points forward.
    fn as_map_blip(&self) -> MapBlip {
        MapBlip { style: self.style, position: self.position, active: self.active }
    }
}

/// The game's name for a style a mod named, or `NotFound` for a style the game does not have.
fn style_named(name: &str) -> BackendResult<&'static str> {
    STYLES.into_iter().find(|style| *style == name).ok_or(BB_ERR_NOT_FOUND)
}

/// The mods' blips as the minimap and the world map take them.
pub fn minimap_blips(bridge: &Bridge) -> Vec<MapBlip> {
    bridge.blips.borrow().iter().map(ModBlip::as_map_blip).collect()
}

impl MapBackend for MapPort {
    fn add(&self, owner: &str, blip: &BlipSpec<'_>) -> BackendResult<u64> {
        let style = style_named(blip.style)?;
        let handle = self.0.next_handle();
        self.0.blips.borrow_mut().push(ModBlip::new(handle, owner, style, blip));
        Ok(handle)
    }

    fn update(&self, owner: &str, handle: u64, blip: &BlipSpec<'_>) -> BackendResult<()> {
        let style = style_named(blip.style)?;
        let mut blips = self.0.blips.borrow_mut();
        let known =
            blips.iter_mut().find(|known| known.handle == handle && known.owner == owner).ok_or(BB_ERR_NOT_FOUND)?;
        *known = ModBlip::new(handle, owner, style, blip);
        Ok(())
    }

    fn remove(&self, owner: &str, handle: u64) -> BackendResult<()> {
        let mut blips = self.0.blips.borrow_mut();
        let before = blips.len();
        blips.retain(|blip| !(blip.handle == handle && blip.owner == owner));
        if blips.len() == before {
            return Err(BB_ERR_NOT_FOUND);
        }
        Ok(())
    }
}
