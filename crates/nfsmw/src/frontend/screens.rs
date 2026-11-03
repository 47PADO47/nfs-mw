//! The stack of loaded screens, the runtime that runs their packages, and the dispatch of what the packages
//! tell the game (docs/specs/frontend-menus.md, section 1).

use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
use blackbox_feng::ObjectRef;
use blackbox_feng::ids::BUTTON_PRESSED;
use blackbox_feng::{Outgoing, PackageId, Runtime};
use glam::Vec2;

use super::confirm::{AlertLogic, ConfirmLogic};
use super::factory;
use super::ids::screen::DIALOG;
use super::logic::{Args, Command, Cx, Env, Pick, ScreenLogic};
use super::look::Look;
use super::world_map::WorldMapFrame;
use crate::ui::theme::{DEFAULT_ASPECT, ElementId};
use crate::ui::{Catalog, UiAssets};

mod aspect;
mod command;
mod geometry;
mod input_gate;
mod layers;
mod load;
mod pause_background;
mod pose;
mod shown;
#[cfg(test)]
mod shown_tests;
mod start;
mod text;
mod trace;
mod vertical;

pub use geometry::ListGeometry;
pub use layers::MenuLayers;
pub use shown::Presented;

struct Screen {
    id: PackageId,
    name: String,
    logic: Box<dyn ScreenLogic>,
    /// The menu slot the screen is drawn for, or `None` when no slot takes it (dialogs, notices). Set once, in `open`.
    slot: Option<ElementId>,
    /// The backdrop the screen wants behind it, or `None` when it keeps whatever is there (`look.rs`).
    look: Option<Look>,
    /// The pose the screen asks the camera for (`factory::pose_for`), or `None` when it keeps the camera where it is.
    pose: Option<&'static str>,
    /// The decorated tree of the last frame, kept while the screen does not change (`screens/shown.rs`).
    shown: shown::Shown,
}

pub struct Screens {
    /// Optional capture telemetry. No tree or event data is built when disabled.
    trace: Option<trace::Trace>,
    runtime: Runtime,
    language: crate::ui::LanguageWatch,
    catalog: Catalog,
    assets: Arc<UiAssets>,
    stack: Vec<Screen>,
    memory: HashMap<String, usize>,
    /// A screen gaining control waits for release before accepting a new gesture.
    await_release: bool,
    /// The key names of the controls list, set by the frame's bindings (`set_key_texts`).
    key_texts: Vec<[String; 2]>,
    /// The world map's frame while the map is open (`set_world_map`).
    world_map: Option<WorldMapFrame>,
    /// The window's width over its height (`set_aspect`): where the left screen edge lies for the vertical menus.
    aspect: f32,
    /// Whose places a click on a list is tested against (`set_list_geometry`).
    geometry: ListGeometry,
    /// Where the pointer was last frame, so a hover follows its movement and not its rest (`hover_row`).
    pointer: Option<Vec2>,
    /// The last frame's pad mask, for `ScreenLogic::pad_pressed`.
    last_pad: u32,
    /// The device whose Start went down this frame, until `update` hands it to the top screen (`screens/start.rs`).
    start_press: Option<crate::input::PlayerDevice>,
}

impl Screens {
    /// The textures and fonts the screens draw with.
    pub fn assets(&self) -> &UiAssets {
        &self.assets
    }

    pub fn new(catalog: Catalog, assets: Arc<UiAssets>) -> Self {
        let runtime = crate::ui::runtime();
        Self {
            trace: trace::Trace::from_environment(),
            runtime,
            language: crate::ui::LanguageWatch::default(),
            catalog,
            assets,
            stack: Vec::new(),
            memory: HashMap::new(),
            await_release: false,
            key_texts: Vec::new(),
            world_map: None,
            aspect: DEFAULT_ASPECT,
            geometry: ListGeometry::default(),
            pointer: None,
            last_pad: 0,
            start_press: None,
        }
    }

    /// Sets the key names the controls list shows this frame (`controls_screen::key_texts`).
    pub fn set_key_texts(&mut self, key_texts: Vec<[String; 2]>) {
        self.key_texts = key_texts;
    }

    /// Sets the world map's frame for the screens this tick (`None` once the map closes).
    pub fn set_world_map(&mut self, frame: Option<WorldMapFrame>) {
        self.world_map = frame;
    }

    /// The file name of the screen on top.
    pub fn top(&self) -> Option<&str> {
        self.stack.last().map(|s| s.name.as_str())
    }

    /// Unloads every screen.
    pub fn clear(&mut self) {
        while let Some(s) = self.stack.pop() {
            self.runtime.unload(s.id);
        }
        self.trace_stack("clear");
    }

    /// Loads a screen on top of the stack; `replace` first unloads the one that is there. Commands the new
    /// screen queues while starting are returned.
    pub fn open(&mut self, name: &str, args: Args, replace: bool, env: &mut Env) -> Vec<Command> {
        let mod_rows = env.mods.is_some_and(crate::modding::Mods::has_mods_page);
        let slot = factory::slot_for(name, &args);
        let (look, pose) = (factory::look_for(name, &args), factory::pose_for(name, &args));
        let logic = factory::make(name, args, mod_rows);
        self.open_as(name, logic, slot, (look, pose), replace, env)
    }

    /// Like [`Self::open`], with the logic already chosen (a confirmation dialog carries its question). Its menu slot is
    /// the one its package has by name (`factory::slot_for`), with no `Args`.
    pub fn open_with(&mut self, name: &str, logic: Box<dyn ScreenLogic>, replace: bool, env: &mut Env) -> Vec<Command> {
        let slot = factory::slot_for(name, &Args::default());
        self.open_as(name, logic, slot, (None, None), replace, env)
    }

    fn open_as(
        &mut self,
        name: &str,
        mut logic: Box<dyn ScreenLogic>,
        slot: Option<ElementId>,
        (look, pose): (Option<Look>, Option<&'static str>),
        replace: bool,
        env: &mut Env,
    ) -> Vec<Command> {
        let Some(package) = self.catalog.find(name).cloned() else {
            log::warn!("no screen {name:?} in the install");
            return Vec::new();
        };
        if replace && let Some(old) = self.stack.pop() {
            self.runtime.unload(old.id);
        }
        self.open_pause_background(name);
        let missing = self.assets.missing_resources(&package);
        if !missing.is_empty() {
            log::debug!("{name}: resources without a texture or font: {missing:?}");
        }
        let file_name = package.name.clone();
        let id = self.runtime.load(package);
        let mut commands = Vec::new();
        let mut cx = Cx {
            rt: &mut self.runtime,
            package: id,
            assets: &self.assets,
            settings: env.settings,
            changed: env.changed,
            caps: env.caps,
            commands: &mut commands,
            memory: &mut self.memory,
            key_texts: &self.key_texts,
            world_map: self.world_map.as_ref(),
            mods: env.mods,
            name: &file_name,
        };
        logic.start(&mut cx);
        aspect::bind(cx.rt, cx.package, cx.name, self.aspect);
        let shown = shown::Shown::default();
        self.stack.push(Screen { id, name: file_name, logic, slot, look, pose, shown });
        self.trace_stack("open");
        self.await_release = true;
        self.give_control();
        commands
    }

    /// Unloads the top screen. The screen below gets its buttons back: the press that opened the screen on top
    /// (a quit question, say) switched them off, as a package does while it leaves.
    pub fn pop(&mut self, env: &mut Env) -> Vec<Command> {
        if let Some(old) = self.stack.pop() {
            self.runtime.unload(old.id);
        }
        self.trace_stack("pop");
        self.await_release = true;
        self.give_control();
        let Self { runtime, assets, stack, memory, key_texts, world_map, aspect, .. } = self;
        let Some(top) = stack.last_mut() else { return Vec::new() };
        runtime.set_input_enabled(top.id, true);
        let (settings, changed, caps, mods) = (&mut *env.settings, &mut *env.changed, env.caps, env.mods);
        let mut commands = Vec::new();
        let mut cx = Cx {
            rt: runtime,
            package: top.id,
            assets,
            settings,
            changed,
            caps,
            commands: &mut commands,
            memory,
            key_texts,
            world_map: world_map.as_ref(),
            mods,
            name: &top.name,
        };
        top.logic.resume(&mut cx);
        aspect::bind(cx.rt, cx.package, cx.name, *aspect);
        commands
    }

    /// Only the screen on top takes pad input.
    fn give_control(&mut self) {
        let top = self.stack.last().map(|s| s.id);
        for s in &self.stack {
            self.runtime.set_control(s.id, !self.await_release && Some(s.id) == top && s.logic.accepts_input());
        }
    }

    /// Advances the runtime by `dt` seconds with the pad `mask`, hands the screens what their packages sent, and
    /// returns what the screens ask for that is not about the stack itself.
    pub fn update(&mut self, dt: f32, mask: u32, env: &mut Env) -> Vec<Command> {
        let host_pad = mask;
        if self.await_release && mask == 0 {
            self.await_release = false;
            self.give_control();
        }
        // Host logic owns pending transitions as well as the package's authored input switches.
        self.give_control();
        let mask = match self.stack.last().is_some_and(|top| top.logic.rotate_pad()) {
            true => vertical::vertical_pad(mask),
            false => mask,
        };
        let mask = if self.top_accepts_input() { mask } else { 0 };
        if let Some(trace) = self.trace.as_mut() {
            trace.begin_frame(dt, host_pad, mask, self.await_release);
        }
        let pressed = mask & !self.last_pad;
        self.last_pad = mask;
        self.runtime.set_pad_mask(mask);
        self.language.apply(&mut self.runtime);
        self.runtime.update(dt);
        let mut commands = Vec::new();
        if let Some(top) = self.stack.last().map(|s| s.id).filter(|_| pressed != 0) {
            self.with_logic(top, env, &mut commands, |logic, cx| logic.pad_pressed(cx, pressed));
        }
        self.deliver_start_press(env, &mut commands);
        let outgoing = self.runtime.take_outgoing();
        if let Some(trace) = self.trace.as_mut() {
            trace.outgoing(&outgoing);
        }
        crate::audio::menu::request_batch(outgoing.iter().filter_map(|out| match out {
            Outgoing::Sound { message, package, .. } if self.sound_allowed(*package, *message) => {
                Some((*package, *message))
            }
            _ => None,
        }));
        for out in outgoing {
            match out {
                Outgoing::Game { message, package, .. } => self.deliver(package, message, env, &mut commands),
                Outgoing::Package { kind, name, .. } => commands.extend(command::package_command(kind, name)),
                // A button that answers BUTTON_PRESSED announces the press to the sound system with its GUID. Only the
                // logic that asks for it hears it (`ScreenLogic::pressed`): some packages post nothing to the game.
                Outgoing::Sound { message: BUTTON_PRESSED, package, from: Some(button) } => {
                    self.deliver_press(package, button, env, &mut commands)
                }
                Outgoing::Sound { .. } => {}
            }
        }
        let ids: Vec<PackageId> = self.stack.iter().map(|s| s.id).collect();
        for id in ids {
            self.deliver_tick(id, dt, env, &mut commands);
        }
        let commands = self.apply_stack_commands(commands, env);
        self.trace_sample();
        commands
    }

    /// Runs `call` on the logic of the screen of `package`, with the context it works in.
    fn with_logic(
        &mut self,
        package: PackageId,
        env: &mut Env,
        out: &mut Vec<Command>,
        call: impl FnOnce(&mut dyn ScreenLogic, &mut Cx),
    ) {
        let Self { runtime, assets, stack, memory, key_texts, world_map, .. } = self;
        let Some(screen) = stack.iter_mut().find(|s| s.id == package) else { return };
        let (settings, changed, caps, mods) = (&mut *env.settings, &mut *env.changed, env.caps, env.mods);
        let name = &screen.name;
        let mut cx = Cx {
            rt: runtime,
            package,
            assets,
            settings,
            changed,
            caps,
            commands: out,
            memory,
            key_texts,
            world_map: world_map.as_ref(),
            mods,
            name,
        };
        call(screen.logic.as_mut(), &mut cx);
    }

    fn deliver(&mut self, package: PackageId, message: u32, env: &mut Env, out: &mut Vec<Command>) {
        self.with_logic(package, env, out, |logic, cx| logic.message(cx, message));
    }

    /// Tells the screen of `package` which button (by GUID) was pressed.
    fn deliver_press(&mut self, package: PackageId, button: u32, env: &mut Env, out: &mut Vec<Command>) {
        self.with_logic(package, env, out, |logic, cx| logic.pressed(cx, button));
    }

    fn deliver_tick(&mut self, package: PackageId, dt: f32, env: &mut Env, out: &mut Vec<Command>) {
        self.with_logic(package, env, out, |logic, cx| logic.tick(cx, dt));
    }

    /// Runs the commands that switch, push or pop screens (which can queue more) and returns the rest.
    fn apply_stack_commands(&mut self, mut commands: Vec<Command>, env: &mut Env) -> Vec<Command> {
        let mut rest = Vec::new();
        let mut budget = 32;
        while !commands.is_empty() && budget > 0 {
            budget -= 1;
            for command in std::mem::take(&mut commands) {
                match command {
                    Command::Switch(name, args) => commands.extend(self.open(&name, args, true, env)),
                    Command::Push(name, args) => commands.extend(self.open(&name, args, false, env)),
                    Command::Pop => commands.extend(self.pop(env)),
                    Command::Confirm(confirm) => {
                        let logic = Box::new(ConfirmLogic::new(confirm));
                        commands.extend(self.open_with(DIALOG, logic, false, env));
                    }
                    Command::EnterName(purpose) => {
                        let logic = Box::new(super::name_entry::NameEntry::new(purpose));
                        commands.extend(self.open_with(DIALOG, logic, false, env));
                    }
                    Command::Alert(alert) => {
                        let logic = Box::new(AlertLogic::new(alert));
                        commands.extend(self.open_with(DIALOG, logic, false, env));
                    }
                    other => rest.push(other),
                }
            }
        }
        rest
    }

    /// Posts a message straight to an object of the top screen (a mouse click, a hot key).
    pub fn post_to_top(&mut self, name_hash: u32, message: u32) {
        if !self.top_accepts_input() {
            return;
        }
        let Some(top) = self.stack.last() else { return };
        let target = self.runtime.find(top.id, name_hash);
        self.runtime.post(top.id, message, target);
    }

    /// Focuses the button of the top screen under a point of the FEng screen (640 x 480, origin at the centre).
    /// Returns false when no button is there.
    pub fn focus_button_at(&mut self, point: Vec2) -> bool {
        if !self.top_accepts_input() || self.top_is_vertical() {
            return false;
        }
        let Some(top) = self.stack.last() else { return false };
        let Some(button) = self.runtime.button_at(top.id, point) else { return false };
        self.runtime.focus_button(button);
        true
    }

    /// Sends a mouse release of `message` to the object of the top screen under a point, when one answers it.
    /// Returns whether a message was sent.
    pub fn release_at(&mut self, point: Vec2, message: u32) -> bool {
        if !self.top_accepts_input() || self.top_is_vertical() {
            return false;
        }
        let Some(top) = self.stack.last() else { return false };
        let Some(target) = self.runtime.mouse_target_at(top.id, point, message) else { return false };
        self.runtime.post(top.id, message, Some(target));
        true
    }

    /// Hands the pointer's position over the top screen to its logic, so the screen can move its focus with it.
    pub fn hover(&mut self, point: Vec2, env: &mut Env) {
        if let Some(row) = self.row_at(point) {
            self.on_top(env, |logic, cx| {
                logic.pick_row(cx, row, Pick::Hover);
                true
            });
            return;
        }
        self.on_top(env, |logic, cx| {
            logic.hover(cx, point);
            true
        });
    }

    /// Hands a click at a point of the top screen to its logic. `None` when the screen does not take it.
    pub fn click(&mut self, point: Vec2, env: &mut Env) -> Option<Vec<Command>> {
        if let Some(row) = self.row_at(point) {
            let (way, along) = (self.way_at(point), self.along_at(point));
            let (_, commands) = self.on_top(env, |logic, cx| {
                logic.pick_row(cx, row, Pick::Press(way, along));
                true
            })?;
            return Some(self.apply_stack_commands(commands, env));
        }
        let (taken, commands) = self.on_top(env, |logic, cx| logic.click(cx, point))?;
        taken.then(|| self.apply_stack_commands(commands, env))
    }

    /// Hands the mouse wheel's notches to the top screen's logic and returns what it queued.
    pub fn scroll(&mut self, steps: i32, env: &mut Env) -> Vec<Command> {
        let Some((_, commands)) = self.on_top(env, |logic, cx| {
            logic.scroll(cx, steps);
            true
        }) else {
            return Vec::new();
        };
        self.apply_stack_commands(commands, env)
    }

    /// Runs `act` on the top screen's logic with its context, and returns what it queued with its answer.
    fn on_top(
        &mut self,
        env: &mut Env,
        act: impl FnOnce(&mut dyn ScreenLogic, &mut Cx) -> bool,
    ) -> Option<(bool, Vec<Command>)> {
        if !self.top_accepts_input() {
            return None;
        }
        let Self { runtime, assets, stack, memory, key_texts, world_map, .. } = self;
        let screen = stack.last_mut()?;
        let (settings, changed, caps, mods) = (&mut *env.settings, &mut *env.changed, env.caps, env.mods);
        let mut commands = Vec::new();
        let mut cx = Cx {
            rt: runtime,
            package: screen.id,
            assets,
            settings,
            changed,
            caps,
            commands: &mut commands,
            memory,
            key_texts,
            world_map: world_map.as_ref(),
            mods,
            name: &screen.name,
        };
        let taken = act(screen.logic.as_mut(), &mut cx);
        Some((taken, commands))
    }

    /// The buttons of the top screen, in file order.
    #[cfg(test)]
    pub fn top_buttons(&self) -> Vec<ObjectRef> {
        self.stack.last().map_or_else(Vec::new, |top| self.runtime.buttons(top.id))
    }

    /// The focused button of the top screen.
    #[cfg(test)]
    pub fn focused_button(&self) -> Option<ObjectRef> {
        self.runtime.current_button(self.stack.last()?.id)
    }
}
