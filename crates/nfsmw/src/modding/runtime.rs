//! The mods the game runs: the runtime, the frame counter, the bridge the mods' calls go through, and the sending of the
//! events.

use std::ffi::c_void;
use std::mem::size_of;
use std::path::PathBuf;
use std::rc::Rc;

use bevy_input::keyboard::KeyCode;
use blackbox_modding::abi::{BB_EVENT_FRAME_START, BB_EVENT_MOD_UNLOADING, BB_EVENT_SCENE_TICK, BbEvent};
use blackbox_modding::host::{DispatchReport, HostOffer, ModRuntime, ModSummary, Policy, Rejected, RunMode};

use super::backends::menus::any_row as has_menu_rows;
use super::backends::{Bridge, Interfaces, commands};
use super::events::{frame_start_payload, interior_payload, scene_tick_payload, unloading_payload};
use super::manager::ModManager;
use super::mw_events;
use super::outbox::InteriorNotice;
use super::pursuit_feed::Notice;
use crate::devtools::Console;
use crate::viewer::Scene;

/// The loaded mods, with the frame counter the events carry, and the bridge their calls read and write. It is a `NonSend`
/// resource: the mods run on the main thread, and the runtime holds `Rc`s.
pub struct Mods {
    runtime: ModRuntime,
    frame: u64,
    bridge: Rc<Bridge>,
    /// The mods the Mods page lists with their switches. Empty until [`Mods::set_manager`] gives it the settings.
    manager: ModManager,
}

impl Mods {
    /// Discover, order and load the mods in `roots` (see [`ModRuntime::start`]), offering the interfaces of `interfaces`.
    /// A mod that fails is recorded, and the game starts without it.
    pub fn start(offer: HostOffer, interfaces: Interfaces, policy: &Policy, roots: &[PathBuf], mode: RunMode) -> Self {
        let Interfaces { registry, bridge } = interfaces;
        let runtime = ModRuntime::start_with(offer, registry, policy, roots, mode);
        Self { runtime, frame: 0, bridge, manager: ModManager::default() }
    }

    /// Give the mod manager the mods of this run and the settings its switches edit (see [`ModManager`]).
    pub fn set_manager(&mut self, manager: ModManager) {
        self.manager = manager;
    }

    /// The mod manager: the listed mods and their switches.
    pub fn manager(&self) -> &ModManager {
        &self.manager
    }

    /// Whether the Mods page has anything to show: a listed mod, or a row a mod registered (`bb.menus`).
    pub fn has_mods_page(&self) -> bool {
        !self.manager.is_empty() || has_menu_rows(&self.bridge)
    }

    /// The loaded mods, in load order.
    pub fn loaded(&self) -> Vec<ModSummary> {
        self.runtime.loaded()
    }

    /// The mods refused, with their reasons.
    pub fn rejected(&self) -> &[Rejected] {
        self.runtime.rejected()
    }

    /// The bridge the mods' calls go through, for the options page's rows.
    pub fn bridge(&self) -> &Bridge {
        &self.bridge
    }

    /// Copy the scene's clock, car, camera and track in, before the mods run this frame. The scene is mutable because
    /// reading the car goes through its physics.
    pub fn sync(&self, scene: &mut dyn Scene) {
        self.bridge.sync(scene);
    }

    /// Give the mods the install, once its layers are in place (the cop waves are read through it).
    pub fn set_install(&self, dir: &game_install::GameDir) {
        self.bridge.set_install(dir.clone());
    }

    /// Fire the hotkeys `pressed` reports as pressed (see [`Bridge::press`]).
    pub fn press(&self, pressed: impl Fn(KeyCode) -> bool) {
        self.bridge.press(pressed);
    }

    /// Make the scene and the console take what the mods asked for (see [`Bridge::apply`]).
    pub fn apply(&self, scene: &mut dyn Scene, console: &mut Console) {
        self.bridge.apply(scene, console);
    }

    /// The mod commands for `help`: (usage, description) pairs.
    pub fn command_help(&self) -> Vec<(String, String)> {
        commands::listing(&self.bridge)
    }

    /// Whether a mod registered a console command called `name` (lower case).
    pub fn has_command(&self, name: &str) -> bool {
        commands::is_registered(&self.bridge, name)
    }

    /// Run a mod's console command (see [`commands::run_mod_command`]).
    pub fn run_command(&self, name: &str, args: &[&str]) -> Result<String, String> {
        commands::run_mod_command(&self.bridge, name, args)
    }

    /// A frame begins (the `ModsPre` set). Counts the frame, then sends `FRAME_START`.
    pub fn frame_start(&mut self, dt_real: f32) -> DispatchReport {
        self.frame += 1;
        self.deliver(BB_EVENT_FRAME_START, &frame_start_payload(dt_real))
    }

    /// The scene has updated (the `ModsPost` set). Sends `SCENE_TICK`.
    pub fn scene_tick(&mut self, dt_real: f32, game_speed: f32, paused: bool) -> DispatchReport {
        self.deliver(BB_EVENT_SCENE_TICK, &scene_tick_payload(dt_real, game_speed, paused))
    }

    /// Send the `nfsmw.*` notices of this frame to every loaded mod, in order (the `ModsPost` set).
    pub fn dispatch_notices(&mut self, notices: &[Notice]) {
        let frame = self.frame;
        let runtime = &mut self.runtime;
        mw_events::send_all(notices, frame, &mut |event| {
            runtime.dispatch_event(event);
        });
    }

    /// Send the interior steps of this frame to every loaded mod, in order (the `ModsPost` set).
    pub fn dispatch_interior(&mut self, notices: &[InteriorNotice]) {
        for notice in notices {
            let payload = interior_payload(&notice.interior, notice.position);
            self.deliver(notice.kind, &payload);
        }
    }

    /// Send one event with a payload to every loaded mod, in load order.
    fn deliver<T>(&mut self, kind: u32, payload: &T) -> DispatchReport {
        let event = BbEvent {
            size: size_of::<BbEvent>() as u32,
            kind,
            frame: self.frame,
            payload: (payload as *const T).cast::<c_void>(),
        };
        self.runtime.dispatch_event(&event)
    }
}

impl Drop for Mods {
    /// Before the libraries close, every mod hears `MOD_UNLOADING` for each mod still loaded, last loaded first. The
    /// runtime then destroys them and closes their libraries.
    fn drop(&mut self) {
        let ids: Vec<String> = self.runtime.loaded().into_iter().rev().map(|summary| summary.id).collect();
        for id in &ids {
            self.deliver(BB_EVENT_MOD_UNLOADING, &unloading_payload(id));
        }
    }
}
