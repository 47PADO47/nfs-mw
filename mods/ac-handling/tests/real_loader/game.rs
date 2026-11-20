//! The fake game of the loader test: every interface the mod uses, the closures the mod registers, and the runtime around it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use blackbox_modding::abi::BB_EVENT_FRAME_START;
use blackbox_modding::abi::{BB_DRIVE_REAR, BB_ERR_DENIED, BB_ERR_NOT_READY, BB_ERR_STALE, BB_ERR_UNSUPPORTED};
use blackbox_modding::host::gate;
use blackbox_modding::host::{
    BackendResult, CommandHandler, CommandsBackend, ConfigBackend, HandlingBackend, HostOffer, HotkeyHandler,
    HudBackend, InputBackend, InterfaceRegistry, MenuHandler, MenuRowSpec, MenusBackend, MessageStyle, ModRuntime,
    OptionSpec, Policy, RunMode, SettingHandler, SettingsBackend, TyreHook, VehicleSpecSheet, VehicleState,
    VehiclesBackend,
};

use crate::build::{LINES, Scratch, install};

pub struct Game {
    pub hooks: RefCell<Vec<(u64, TyreHook)>>,
    pub register_calls: Cell<u32>,
    pub online: Cell<bool>,
    pub spec_revision: Cell<u32>,
    pub mass: Cell<f32>,
    pub settings: RefCell<Vec<(String, Option<SettingHandler>)>>,
    pub values: RefCell<Vec<(String, String)>>,
    pub commands: RefCell<Vec<(String, CommandHandler)>>,
    pub hotkeys: RefCell<Vec<(String, String, HotkeyHandler)>>,
    pub menus: RefCell<Vec<(String, MenuHandler, f32)>>,
    pub hud: RefCell<Vec<String>>,
}

impl Game {
    pub fn new() -> Rc<Game> {
        Rc::new(Game {
            hooks: RefCell::default(),
            register_calls: Cell::new(0),
            online: Cell::new(false),
            spec_revision: Cell::new(1),
            mass: Cell::new(1450.0),
            settings: RefCell::default(),
            values: RefCell::default(),
            commands: RefCell::default(),
            hotkeys: RefCell::default(),
            menus: RefCell::default(),
            hud: RefCell::default(),
        })
    }

    pub fn command(&self, words: &[&str]) -> String {
        let _scope = gate::enter();
        let commands = self.commands.borrow();
        let (_, handler) =
            commands.iter().find(|(name, _)| name == "ac-handling").expect("the mod registered its command");
        handler(words).expect("the command answers")
    }

    pub fn press_hotkey(&self) {
        let _scope = gate::enter();
        let hotkeys = self.hotkeys.borrow();
        let (_, _, press) = hotkeys.first().expect("the mod registered a hotkey");
        press();
    }

    pub fn choose(&self, option: &str, value: &str) {
        let _scope = gate::enter();
        let settings = self.settings.borrow();
        let (_, handler) = settings.iter().find(|(id, _)| id == option).expect("the option exists");
        self.values.borrow_mut().retain(|(id, _)| id != option);
        self.values.borrow_mut().push((option.to_owned(), value.to_owned()));
        handler.as_ref().expect("a handler")(value);
    }

    pub fn hook(&self) -> TyreHook {
        self.hooks.borrow().first().expect("the mod registered a hook").1.clone()
    }
}

impl HandlingBackend for Game {
    fn register_tyre(&self, owner: &str, hook: TyreHook) -> BackendResult<u64> {
        self.register_calls.set(self.register_calls.get() + 1);
        if self.online.get() {
            log::warn!("mod {owner}: bb.handling refused in an online session");
            return Err(BB_ERR_DENIED);
        }
        let handle = u64::from(self.register_calls.get());
        self.hooks.borrow_mut().push((handle, hook));
        Ok(handle)
    }

    fn unregister_tyre(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        let mut hooks = self.hooks.borrow_mut();
        let index = hooks.iter().position(|(known, _)| *known == handle).ok_or(BB_ERR_STALE)?;
        hooks.remove(index).1.retire();
        Ok(())
    }
}

impl VehiclesBackend for Game {
    fn state(&self, _owner: &str) -> BackendResult<VehicleState> {
        Err(BB_ERR_NOT_READY)
    }

    fn teleport(&self, _owner: &str, _position: [f32; 3], _heading: f32) -> BackendResult<()> {
        Err(BB_ERR_NOT_READY)
    }

    fn repair(&self, _owner: &str) -> BackendResult<()> {
        Err(BB_ERR_NOT_READY)
    }

    fn spawn_traffic(&self, _owner: &str, _position: [f32; 3], _heading: f32) -> BackendResult<u64> {
        Err(BB_ERR_UNSUPPORTED)
    }

    fn add_nos(&self, _owner: &str, _amount: f32) -> BackendResult<()> {
        Err(BB_ERR_NOT_READY)
    }

    fn spec(&self, _owner: &str) -> BackendResult<VehicleSpecSheet> {
        let mut gear_ratios = [0.0; 8];
        gear_ratios[..6].copy_from_slice(&[3.6, 2.2, 1.6, 1.25, 1.0, 0.82]);
        Ok(VehicleSpecSheet {
            revision: self.spec_revision.get(),
            mass_kg: self.mass.get(),
            wheelbase_m: 2.45,
            track_m: [1.52, 1.50],
            cg_height_m: 0.5,
            front_weight_fraction: 0.52,
            tyre_radius_m: [0.33, 0.34],
            tyre_width_m: [0.235, 0.265],
            peak_friction: [2.075; 2],
            sliding_friction: [1.85; 2],
            drive_layout: BB_DRIVE_REAR,
            gear_count: 6,
            max_power_w: 300_000.0,
            max_power_rpm: 7000.0,
            redline_rpm: 8000.0,
            final_drive: 3.5,
            gear_ratios,
            brake_torque_nm: [2000.0, 800.0],
            drag_area_m2: 0.65,
            downforce_area_m2: 0.0,
        })
    }
}

impl ConfigBackend for Game {
    fn main_section(&self, _owner: &str) -> BackendResult<Option<String>> {
        Ok(None)
    }
}

impl SettingsBackend for Game {
    fn add_option(&self, _owner: &str, spec: &OptionSpec<'_>, on_change: Option<SettingHandler>) -> BackendResult<u64> {
        let first = spec.values.first().map(|value| value.id.to_owned());
        let initial = if spec.values.is_empty() {
            (if spec.default_on { "on" } else { "off" }).to_owned()
        } else {
            first.unwrap_or_default()
        };
        self.values.borrow_mut().push((spec.id.to_owned(), initial));
        self.settings.borrow_mut().push((spec.id.to_owned(), on_change));
        Ok(self.settings.borrow().len() as u64)
    }

    fn remove_option(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        Ok(())
    }

    fn get(&self, _owner: &str, option: &str) -> BackendResult<String> {
        let values = self.values.borrow();
        values.iter().rev().find(|(id, _)| id == option).map(|(_, value)| value.clone()).ok_or(BB_ERR_STALE)
    }
}

impl MenusBackend for Game {
    fn register(&self, _owner: &str, row: &MenuRowSpec<'_>, on_change: MenuHandler) -> BackendResult<u64> {
        self.menus.borrow_mut().push((row.name.to_owned(), on_change, row.default));
        Ok(self.menus.borrow().len() as u64)
    }

    fn unregister(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        Ok(())
    }

    fn get(&self, _owner: &str, handle: u64) -> BackendResult<f32> {
        let menus = self.menus.borrow();
        menus.get((handle as usize).wrapping_sub(1)).map(|(_, _, value)| *value).ok_or(BB_ERR_STALE)
    }

    fn set(&self, _owner: &str, _handle: u64, _value: f32) -> BackendResult<()> {
        Ok(())
    }
}

impl InputBackend for Game {
    fn register_hotkey(
        &self,
        _owner: &str,
        name: &str,
        _label: &str,
        default_binding: &str,
        on_press: HotkeyHandler,
    ) -> BackendResult<u64> {
        self.hotkeys.borrow_mut().push((name.to_owned(), default_binding.to_owned(), on_press));
        Ok(self.hotkeys.borrow().len() as u64)
    }

    fn unregister(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        Ok(())
    }
}

impl CommandsBackend for Game {
    fn run(&self, _owner: &str, _line: &str) -> BackendResult<String> {
        Err(BB_ERR_UNSUPPORTED)
    }

    fn register(&self, _owner: &str, name: &str, _help: &str, handler: CommandHandler) -> BackendResult<u64> {
        self.commands.borrow_mut().push((name.to_owned(), handler));
        Ok(self.commands.borrow().len() as u64)
    }

    fn unregister(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        Ok(())
    }
}

impl HudBackend for Game {
    fn message(&self, _owner: &str, text: &str, _style: MessageStyle, _secs: f32) -> BackendResult<()> {
        self.hud.borrow_mut().push(text.to_owned());
        Ok(())
    }

    fn hide(&self, _owner: &str) -> BackendResult<()> {
        Ok(())
    }
}

/// Which interfaces the fake game offers beside the two the mod requires.
#[derive(Clone, Copy)]
pub struct Offer {
    pub switches: bool,
}

/// A started runtime with the mod loaded, and the fake game behind it.
pub struct Rig {
    pub runtime: ModRuntime,
    pub game: Rc<Game>,
    pub frame: u64,
    pub _scratch: Scratch,
}

impl Rig {
    pub fn new(tag: &str, config: Option<&str>, offer: Offer) -> Rig {
        let scratch = Scratch::new(tag);
        install(&scratch.path, config);
        let game = Game::new();
        let mut registry = InterfaceRegistry::new();
        registry.add_handling(game.clone()).add_vehicles(game.clone()).add_config(game.clone());
        if offer.switches {
            registry
                .add_settings(game.clone())
                .add_menus(game.clone())
                .add_input(game.clone())
                .add_commands(game.clone())
                .add_hud(game.clone());
        }
        let runtime = ModRuntime::start_with(
            HostOffer::new("nfsmw", "test"),
            registry,
            &Policy::All,
            std::slice::from_ref(&scratch.path),
            RunMode::Normal,
        );
        Rig { runtime, game, frame: 0, _scratch: scratch }
    }

    /// One frame start: the mods get `FRAME_START`, where the switches are applied.
    pub fn frame(&mut self) {
        self.frame += 1;
        self.runtime.dispatch(BB_EVENT_FRAME_START, self.frame);
    }
}

impl Drop for Rig {
    /// A failing test shows what the mod and the host logged.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            return;
        }
        if let Ok(lines) = LINES.lock() {
            eprintln!(
                "--- the log of the failed test ---
{}",
                lines.join(
                    "
"
                )
            );
        }
    }
}

pub const ALL: Offer = Offer { switches: true };
