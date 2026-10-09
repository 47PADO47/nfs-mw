//! The stack of loaded screens, the runtime that runs their packages, and the dispatch of what the packages
//! tell the game (docs/specs/frontend-menus.md, section 1).

use std::collections::HashMap;
use std::sync::Arc;

use blackbox_feng::{Outgoing, PackageCommandKind, PackageId, Runtime, UiTree};

use super::factory;
use super::logic::{Args, Command, Cx, Env, ScreenLogic};
use crate::ui::{Catalog, UiAssets};

struct Screen {
    id: PackageId,
    name: String,
    logic: Box<dyn ScreenLogic>,
}

pub struct Screens {
    runtime: Runtime,
    catalog: Catalog,
    assets: Arc<UiAssets>,
    stack: Vec<Screen>,
    memory: HashMap<String, usize>,
    /// A screen gaining control waits for release before accepting a new gesture.
    await_release: bool,
}

impl Screens {
    pub fn new(catalog: Catalog, assets: Arc<UiAssets>) -> Self {
        let mut runtime = Runtime::new();
        let strings = assets.clone();
        runtime.set_string_resolver(move |label| strings.strings.as_ref()?.get(label));
        Self { runtime, catalog, assets, stack: Vec::new(), memory: HashMap::new(), await_release: false }
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
        let mut logic = factory::make(&file_name, args);
        let mut commands = Vec::new();
        let mut cx = Cx {
            rt: &mut self.runtime,
            package: id,
            assets: &self.assets,
            settings: env.settings,
            changed: env.changed,
            commands: &mut commands,
            memory: &mut self.memory,
            name: &file_name,
        };
        logic.start(&mut cx);
        self.stack.push(Screen { id, name: file_name, logic });
        self.await_release = true;
        self.give_control();
        commands
    }

    /// Unloads the top screen.
    pub fn pop(&mut self) {
        if let Some(old) = self.stack.pop() {
            self.runtime.unload(old.id);
        }
        self.await_release = true;
        self.give_control();
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
        let Self { runtime, assets, stack, memory, .. } = self;
        let Some(screen) = stack.iter_mut().find(|s| s.id == package) else { return };
        let (settings, changed) = (&mut *env.settings, &mut *env.changed);
        let mut cx = Cx { rt: runtime, package, assets, settings, changed, commands: out, memory, name: &screen.name };
        screen.logic.message(&mut cx, message);
    }

    fn deliver_tick(&mut self, package: PackageId, dt: f32, env: &mut Env, out: &mut Vec<Command>) {
        let Self { runtime, assets, stack, memory, .. } = self;
        let Some(screen) = stack.iter_mut().find(|s| s.id == package) else { return };
        let (settings, changed) = (&mut *env.settings, &mut *env.changed);
        let mut cx = Cx { rt: runtime, package, assets, settings, changed, commands: out, memory, name: &screen.name };
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
                    Command::Pop => self.pop(),
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

    pub fn presented_trees(&self, bindings: &crate::input::Bindings, device: crate::input::InputDevice) -> Vec<UiTree> {
        let mut trees = self.trees();
        for (tree, screen) in trees.iter_mut().zip(&self.stack) {
            super::prompts::decorate(tree, &screen.name, bindings, device, &self.assets);
        }
        trees
    }

    /// Posts a message straight to an object of the top screen (a mouse click, a hot key).
    pub fn post_to_top(&mut self, name_hash: u32, message: u32) {
        let Some(top) = self.stack.last() else { return };
        let target = self.runtime.find(top.id, name_hash);
        self.runtime.post(top.id, message, target);
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
