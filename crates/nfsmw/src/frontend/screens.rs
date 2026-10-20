//! The stack of loaded screens, the runtime that runs their packages, and the dispatch of what the packages
//! tell the game (docs/specs/frontend-menus.md, section 1).

use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
use blackbox_feng::ObjectRef;
use blackbox_feng::{Outgoing, PackageCommandKind, PackageId, Runtime, UiTree};
use glam::Vec2;

use super::confirm::{AlertLogic, ConfirmLogic};
use super::factory;
use super::ids::screen::DIALOG;
use super::logic::{Args, Command, Cx, Env, ScreenLogic};
use super::world_map::WorldMapFrame;
use crate::ui::theme::remastered::menu::layout as menu_layout;
use crate::ui::theme::{DEFAULT_ASPECT, ElementId, MenuList};
use crate::ui::{Catalog, UiAssets};

mod vertical;

struct Screen {
    id: PackageId,
    name: String,
    logic: Box<dyn ScreenLogic>,
    /// The menu slot the screen is drawn for, or `None` when no slot takes it (dialogs, notices). Set once, in `open`.
    slot: Option<ElementId>,
}

/// One stack screen's tree for the frame, with the menu slot it is drawn for.
pub struct Presented {
    pub tree: UiTree,
    pub slot: Option<ElementId>,
    /// The screen's entries, for a theme that lays them out itself (icon menus only).
    pub list: Option<MenuList>,
}

pub struct Screens {
    runtime: Runtime,
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
    /// Where the pointer was last frame, so a hover follows its movement and not its rest (`hover_row`).
    pointer: Option<Vec2>,
}

impl Screens {
    pub fn new(catalog: Catalog, assets: Arc<UiAssets>) -> Self {
        let mut runtime = crate::ui::runtime();
        let strings = assets.clone();
        runtime.set_string_resolver(move |label| strings.strings.as_ref()?.get(label));
        Self {
            runtime,
            catalog,
            assets,
            stack: Vec::new(),
            memory: HashMap::new(),
            await_release: false,
            key_texts: Vec::new(),
            world_map: None,
            aspect: DEFAULT_ASPECT,
            pointer: None,
        }
    }

    /// Sets the window's width over its height, for the vertical menus' places.
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
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
    }

    /// Loads a screen on top of the stack; `replace` first unloads the one that is there. Commands the new
    /// screen queues while starting are returned.
    pub fn open(&mut self, name: &str, args: Args, replace: bool, env: &mut Env) -> Vec<Command> {
        let mod_rows = env.mods.is_some_and(crate::modding::Mods::has_mods_page);
        let slot = factory::slot_for(name, &args);
        let logic = factory::make(name, args, mod_rows);
        self.open_as(name, logic, slot, replace, env)
    }

    /// Like [`Self::open`], with the logic already chosen (a confirmation dialog carries its question). Such a screen
    /// has no menu slot.
    pub fn open_with(&mut self, name: &str, logic: Box<dyn ScreenLogic>, replace: bool, env: &mut Env) -> Vec<Command> {
        self.open_as(name, logic, None, replace, env)
    }

    fn open_as(
        &mut self,
        name: &str,
        mut logic: Box<dyn ScreenLogic>,
        slot: Option<ElementId>,
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
        self.stack.push(Screen { id, name: file_name, logic, slot });
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
        self.await_release = true;
        self.give_control();
        let Self { runtime, assets, stack, memory, key_texts, world_map, .. } = self;
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
        commands
    }

    /// Only the screen on top takes pad input.
    fn give_control(&mut self) {
        let top = self.stack.last().map(|s| s.id);
        for s in &self.stack {
            self.runtime.set_control(s.id, !self.await_release && Some(s.id) == top);
        }
    }

    /// Advances the runtime by `dt` seconds with the pad `mask`, hands the screens what their packages sent, and
    /// returns what the screens ask for that is not about the stack itself.
    pub fn update(&mut self, dt: f32, mask: u32, env: &mut Env) -> Vec<Command> {
        if self.await_release && mask == 0 {
            self.await_release = false;
            self.give_control();
        }
        let mask = match self.stack.last().is_some_and(|top| top.logic.vertical()) {
            true => vertical::vertical_pad(mask),
            false => mask,
        };
        self.runtime.set_pad_mask(match self.await_release {
            true => 0,
            false => mask,
        });
        self.runtime.update(dt);
        let mut commands = Vec::new();
        for out in self.runtime.take_outgoing() {
            match out {
                Outgoing::Game { message, package, .. } => self.deliver(package, message, env, &mut commands),
                Outgoing::Package { kind, name, .. } => commands.extend(package_command(kind, name)),
                Outgoing::Sound { .. } => {}
            }
        }
        let ids: Vec<PackageId> = self.stack.iter().map(|s| s.id).collect();
        for id in ids {
            self.deliver_tick(id, dt, env, &mut commands);
        }
        self.apply_stack_commands(commands, env)
    }

    fn deliver(&mut self, package: PackageId, message: u32, env: &mut Env, out: &mut Vec<Command>) {
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
        screen.logic.message(&mut cx, message);
    }

    fn deliver_tick(&mut self, package: PackageId, dt: f32, env: &mut Env, out: &mut Vec<Command>) {
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
        screen.logic.tick(&mut cx, dt);
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

    /// The trees to draw, lowest screen first.
    pub fn trees(&self) -> Vec<UiTree> {
        self.stack.iter().map(|s| self.runtime.tree(s.id)).collect()
    }

    /// The trees to draw this frame, lowest first, each with its menu slot. The decorated trees are what the presenter
    /// gets, so the menu cell (`ui/theme/original/frame.rs`) holds the same ones.
    pub fn presented(&self, bindings: &crate::input::Bindings, device: crate::input::InputDevice) -> Vec<Presented> {
        let mut trees = self.trees();
        for (tree, screen) in trees.iter_mut().zip(&self.stack) {
            super::prompts::decorate(tree, &screen.name, bindings, device, &self.assets);
        }
        trees
            .into_iter()
            .zip(&self.stack)
            .map(|(tree, screen)| Presented { tree, slot: screen.slot, list: self.list_of(screen, bindings, device) })
            .collect()
    }

    /// The trees alone, lowest first (the tests read them; the presenter takes [`Self::presented`]).
    #[cfg(test)]
    pub fn presented_trees(&self, bindings: &crate::input::Bindings, device: crate::input::InputDevice) -> Vec<UiTree> {
        self.presented(bindings, device).into_iter().map(|presented| presented.tree).collect()
    }

    /// Posts a message straight to an object of the top screen (a mouse click, a hot key).
    pub fn post_to_top(&mut self, name_hash: u32, message: u32) {
        let Some(top) = self.stack.last() else { return };
        let target = self.runtime.find(top.id, name_hash);
        self.runtime.post(top.id, message, target);
    }

    /// Focuses the button of the top screen under a point of the FEng screen (640 x 480, origin at the centre).
    /// Returns false when no button is there.
    pub fn focus_button_at(&mut self, point: Vec2) -> bool {
        if self.top_is_vertical() {
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
        if self.top_is_vertical() {
            return false;
        }
        let Some(top) = self.stack.last() else { return false };
        let Some(target) = self.runtime.mouse_target_at(top.id, point, message) else { return false };
        self.runtime.post(top.id, message, Some(target));
        true
    }

    /// Hands the pointer's position over the top screen to its logic, so the screen can move its focus with it.
    pub fn hover(&mut self, point: Vec2, env: &mut Env) {
        self.on_top(env, |logic, cx| {
            logic.hover(cx, point);
            true
        });
    }

    /// Hands a click at a point of the top screen to its logic. `None` when the screen does not take it.
    pub fn click(&mut self, point: Vec2, env: &mut Env) -> Option<Vec<Command>> {
        if let Some(row) = self.row_at(point) {
            let (_, commands) = self.on_top(env, |logic, cx| {
                logic.pick_row(cx, row, true);
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

    /// The footer hint of the top screen under a point that a click presses, as the screen draws it now.
    pub fn hint_at(
        &self,
        point: Vec2,
        bindings: &crate::input::Bindings,
        device: crate::input::InputDevice,
    ) -> Option<crate::input::Action> {
        let top = self.stack.last()?;
        if top.logic.vertical() {
            let list = self.list_of(top, bindings, device)?;
            return menu_layout::hint_at(&list.hints, point, self.aspect);
        }
        let mut tree = self.runtime.tree(top.id);
        let hints = super::prompts::decorate(&mut tree, &top.name, bindings, device, &self.assets);
        hints.iter().find(|h| h.covers(point)).map(|h| h.action)
    }

    /// Runs `act` on the top screen's logic with its context, and returns what it queued with its answer.
    fn on_top(
        &mut self,
        env: &mut Env,
        act: impl FnOnce(&mut dyn ScreenLogic, &mut Cx) -> bool,
    ) -> Option<(bool, Vec<Command>)> {
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

fn package_command(kind: PackageCommandKind, name: String) -> Option<Command> {
    match kind {
        PackageCommandKind::Switch => Some(Command::Switch(name, Args::default())),
        PackageCommandKind::PushGlobal | PackageCommandKind::PushCurrent | PackageCommandKind::PushNone => {
            Some(Command::Push(name, Args::default()))
        }
        PackageCommandKind::Pop => Some(Command::Pop),
        PackageCommandKind::RecordMarker | PackageCommandKind::SwitchToMarker | PackageCommandKind::ClearMarkers => {
            None
        }
    }
}
