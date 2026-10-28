//! The per-frame context the mods' calls read and write.
//!
//! The ports in this folder read it and queue their writes in it, all on the main thread, inside a mod callback. The frame
//! systems in `plugin.rs` do the rest: [`Bridge::sync`] copies the scene's clock, car, camera and track in before the mods
//! run, [`Bridge::press`] fires the hotkeys that were pressed, and [`Bridge::apply`] makes the scene and the console take
//! what the mods asked for. Nothing here is a global: the bridge belongs to the `Mods` resource, and the ports hold an `Rc`
//! to it.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use bevy_input::keyboard::KeyCode;
use blackbox_attrib::Database;
use blackbox_modding::content::{
    NitrousProfile, SparksProfile, TireSmokeProfile, WindProfile, XenonProfile, parse_nitrous_profile,
    parse_sparks_profile, parse_tire_smoke_profile, parse_wind_profile, parse_xenon_profile,
};
use blackbox_modding::host::{HotkeyHandler, OptionsModel};
use blackbox_ui::ThemeId;
use game_install::GameDir;
use nfsmw_data::pursuit::{HeatRow, heat_row, race_heat_row};
use nfsmw_modding::abi::CopTable;

use super::commands::ModCommand;
use super::input::Hotkey;
use super::map::{self, ModBlip};
use super::menu_store::{ConfigFile, ValueStore};
use super::menus::MenuRow;
use super::settings::{self, OptionHandler};
use super::time::{self, Clock, ClockRequest};
use super::ui::UiRecord;
use super::variants::{COLLISION_SPARKS_KIND, CacheCell, NITROUS_KIND, TIRE_SMOKE_KIND, WIND_KIND, XENON_SPARKS_KIND};
use super::world::{self, WorldRequest};
use crate::devtools::Console;
use crate::effect_variants::EffectVariants;
use crate::viewer::{CameraReadout, PlayerCar, Scene, WorldInfo};

pub struct Bridge {
    /// The last handle given out. Handles start at 1; 0 means none.
    last_handle: Cell<u64>,
    /// Console commands the mods registered, in registration order.
    pub(super) commands: RefCell<Vec<ModCommand>>,
    /// Console lines the mods asked to run (`bb.commands.run`), handed to the console by [`Bridge::apply`].
    pub(super) queued_lines: RefCell<Vec<String>>,
    /// Hotkeys the mods registered.
    pub(super) hotkeys: RefCell<Vec<Hotkey>>,
    /// The day clock as the mods see it this frame, or `None` when the scene has no clock (the menus).
    pub(super) clock: Cell<Option<Clock>>,
    /// Clock changes the mods asked for, applied to the scene in order by [`Bridge::apply`].
    pub(super) clock_requests: RefCell<Vec<ClockRequest>>,
    /// The player's car as the mods see it this frame, or `None` when no car is driven.
    pub(super) vehicle: Cell<Option<PlayerCar>>,
    /// The player's engine, pedals and car id as the mods see them this frame (`bb.engine`), or `None` when no car is driven.
    pub(super) engine: RefCell<Option<crate::engine_switch::EngineReadout>>,
    /// The mod that stopped the engine and has not started it again: the engine runs when that mod unloads.
    pub(super) engine_holder: RefCell<Option<String>>,
    /// The camera as the mods see it this frame, or `None` where the scene has none.
    pub(super) camera: Cell<Option<CameraReadout>>,
    /// The track and the district this frame, or `None` where no world is loaded.
    pub(super) world: RefCell<Option<WorldInfo>>,
    /// Changes to the car, the camera, the weather, the traffic and the HUD message the mods asked for, applied to the
    /// scene in order by [`Bridge::apply`].
    pub(super) world_requests: RefCell<Vec<WorldRequest>>,
    /// The minimap blips the mods placed.
    pub(super) blips: RefCell<Vec<ModBlip>>,
    /// The rows the mods added to the Mods page of the options.
    pub(super) menu_rows: RefCell<Vec<MenuRow>>,
    /// The options of the gameplay settings: the game's own and what the mods added (`bb.settings`).
    pub(super) options: RefCell<OptionsModel>,
    /// The callbacks the mods gave for their options.
    pub(super) option_handlers: RefCell<Vec<OptionHandler>>,
    /// The chosen value of each option read so far, so the config file is not read every frame.
    pub(super) option_choices: RefCell<HashMap<String, String>>,
    /// The profile of the nitrous variant in effect, read from the install once per chosen value (`nitrous_profile`).
    pub(super) nitrous_cache: CacheCell<NitrousProfile>,
    /// The same for the tire smoke variant (`tire_smoke_profile`).
    pub(super) tire_smoke_cache: CacheCell<TireSmokeProfile>,
    /// The same for the collision spark variant (`collision_sparks_profile`) and the restored spark variant (`xenon_sparks_profile`).
    pub(super) collision_sparks_cache: CacheCell<SparksProfile>,
    pub(super) xenon_sparks_cache: CacheCell<XenonProfile>,
    /// The same for the wind variant (`wind_profile`).
    pub(super) wind_cache: CacheCell<WindProfile>,
    /// The themes the mods registered (`bb.ui`), with the draw specs of their slots.
    pub(super) ui_themes: RefCell<Vec<UiRecord>>,
    /// The mod theme the player chose on the Mods page, or `None` for the built-in style (`ui.rs`). The theme sync
    /// reads it each frame and keeps it while the theme is registered.
    pub(super) ui_theme_choice: Cell<Option<ThemeId>>,
    /// Where the rows' values are saved.
    pub(super) store: Box<dyn ValueStore>,
    /// The install, once the layers are in place (see `open.rs`). The cop waves of the install are read through it.
    install: RefCell<Option<GameDir>>,
}

impl Default for Bridge {
    /// The bridge of a game run: the rows' values go to the config file.
    fn default() -> Self {
        Self::with_store(Box::new(ConfigFile))
    }
}

impl Bridge {
    /// A bridge whose rows save their values in `store`.
    pub fn with_store(store: Box<dyn ValueStore>) -> Self {
        Self {
            last_handle: Cell::new(0),
            commands: RefCell::default(),
            queued_lines: RefCell::default(),
            hotkeys: RefCell::default(),
            clock: Cell::new(None),
            clock_requests: RefCell::default(),
            vehicle: Cell::new(None),
            engine: RefCell::default(),
            engine_holder: RefCell::default(),
            camera: Cell::new(None),
            world: RefCell::default(),
            world_requests: RefCell::default(),
            blips: RefCell::default(),
            menu_rows: RefCell::default(),
            options: RefCell::new(OptionsModel::new(settings::builtin())),
            option_handlers: RefCell::default(),
            option_choices: RefCell::default(),
            nitrous_cache: RefCell::default(),
            tire_smoke_cache: RefCell::default(),
            collision_sparks_cache: RefCell::default(),
            xenon_sparks_cache: RefCell::default(),
            wind_cache: RefCell::default(),
            ui_themes: RefCell::default(),
            ui_theme_choice: Cell::new(None),
            store,
            install: RefCell::default(),
        }
    }

    /// A handle nobody has been given yet.
    pub(super) fn next_handle(&self) -> u64 {
        let handle = self.last_handle.get() + 1;
        self.last_handle.set(handle);
        handle
    }

    /// Give the bridge the install, with its layers in place. The cop waves read the install's attributes through it.
    pub fn set_install(&self, dir: GameDir) {
        *self.install.borrow_mut() = Some(dir);
    }

    /// A copy of the install, with its layers (late ones included: they are shared by every copy). `None` until known.
    pub(super) fn install(&self) -> Option<GameDir> {
        self.install.borrow().clone()
    }

    /// The install's wave of `table` at `level`, as the install defines it. `NotReady` until the install is known,
    /// `NotFound` for a level the install has no wave for.
    pub(super) fn install_row(&self, table: CopTable, level: u32) -> Result<HeatRow, blackbox_modding::abi::BbStatus> {
        let db = self.database()?;
        let row = match table {
            CopTable::Roam => heat_row(&db, level),
            CopTable::Race => race_heat_row(&db, level),
        };
        row.ok_or(blackbox_modding::abi::BB_ERR_NOT_FOUND)
    }

    /// The install's attribute database, shared by the session (see `nfsmw_data::shared::attributes`).
    fn database(&self) -> Result<Arc<Database>, blackbox_modding::abi::BbStatus> {
        let install = self.install.borrow();
        let dir = install.as_ref().ok_or(blackbox_modding::abi::BB_ERR_NOT_READY)?;
        nfsmw_data::shared::attributes(dir).map_err(|_| blackbox_modding::abi::BB_ERR_HOST)
    }

    /// The effects of the nitrous variant the player chose, when it is a mod's (`variant_profile`): `NITROUS/<value>/profile.toml`.
    pub fn nitrous_profile(&self) -> Option<Arc<NitrousProfile>> {
        self.variant_profile(&NITROUS_KIND, &self.nitrous_cache, parse_nitrous_profile)
    }

    /// The look of the tire smoke variant the player chose, when it is a mod's: `TIRESMOKE/<value>/profile.toml`.
    pub fn tire_smoke_profile(&self) -> Option<Arc<TireSmokeProfile>> {
        self.variant_profile(&TIRE_SMOKE_KIND, &self.tire_smoke_cache, parse_tire_smoke_profile)
    }

    /// The collision sparks of the variant the player chose, when it is a mod's: `SPARKS/<value>/profile.toml`.
    pub fn collision_sparks_profile(&self) -> Option<Arc<SparksProfile>> {
        self.variant_profile(&COLLISION_SPARKS_KIND, &self.collision_sparks_cache, parse_sparks_profile)
    }

    /// The restored line sparks of the variant the player chose, when it is a mod's: `XENON/<value>/profile.toml`.
    pub fn xenon_sparks_profile(&self) -> Option<Arc<XenonProfile>> {
        self.variant_profile(&XENON_SPARKS_KIND, &self.xenon_sparks_cache, parse_xenon_profile)
    }

    /// The wind variant the player chose, when it is a mod's: `WIND/<value>/profile.toml`.
    pub fn wind_profile(&self) -> Option<Arc<WindProfile>> {
        self.variant_profile(&WIND_KIND, &self.wind_cache, parse_wind_profile)
    }

    /// The profiles of every kind of variant the player chose, for the scene (`EffectVariants`). A new kind adds its field here.
    pub fn effect_variants(&self) -> EffectVariants {
        EffectVariants {
            nitrous: self.nitrous_profile(),
            tire_smoke: self.tire_smoke_profile(),
            collision_sparks: self.collision_sparks_profile(),
            xenon_sparks: self.xenon_sparks_profile(),
            wind: self.wind_profile(),
        }
    }

    /// Copy the scene's clock, car, camera and track in, so the mods read this frame's state. Called before the mods run.
    pub fn sync(&self, scene: &mut dyn Scene) {
        self.clock.set(time::read(scene));
        self.vehicle.set(scene.player_car());
        *self.engine.borrow_mut() = scene.engine_readout();
        self.camera.set(scene.camera_readout());
        *self.world.borrow_mut() = scene.world_info();
    }

    /// Fire the hotkeys `pressed` reports as pressed this frame. The callbacks run one by one, with no borrow of the
    /// table held, so a callback may register or remove hotkeys.
    pub fn press(&self, pressed: impl Fn(KeyCode) -> bool) {
        let due: Vec<Rc<HotkeyHandler>> = self
            .hotkeys
            .borrow()
            .iter()
            .filter(|hotkey| hotkey.key.is_some_and(&pressed))
            .map(|hotkey| hotkey.handler.clone())
            .collect();
        for handler in due {
            (*handler)();
        }
    }

    /// Make the scene and the console take what the mods asked for this frame: the clock changes, the world changes, the
    /// blips, and the console lines, which run from the next frame on.
    pub fn apply(&self, scene: &mut dyn Scene, console: &mut Console) {
        let requests = std::mem::take(&mut *self.clock_requests.borrow_mut());
        time::apply(requests, scene);
        let requests = std::mem::take(&mut *self.world_requests.borrow_mut());
        world::apply(requests, scene);
        scene.set_mod_blips(&map::minimap_blips(self));
        scene.set_effect_variants(&self.effect_variants());
        let lines = std::mem::take(&mut *self.queued_lines.borrow_mut());
        console.queue_lines(lines);
    }
}
