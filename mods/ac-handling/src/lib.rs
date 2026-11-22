//! `ac-handling`: an Assetto Corsa-style handling mod for the player's car of NFS: Most Wanted.
//!
//! It targets the *feel* of Assetto Corsa with a model written from our own specs (`docs/specs/ac-handling/`): a slip curve
//! with a peak and a fall-off, load sensitivity, one grip budget for both directions, a wheel integrated in substeps inside the
//! tyre hook (`bb.handling`), and, as optional units, tyre temperature and wear, a suspension with a hub, driver aids and a
//! steering model. It is an independent project, not affiliated with or endorsed by Kunos Simulazioni or 505 Games, and holds no
//! game file and no foreign code.
//!
//! The crate is a set of pure modules and a thin glue:
//!
//! - [`tyre_force`], [`wheel`] and [`substep`]: the tyre model (tyre-force.md and wheel-dynamics.md);
//! - [`thermal`] and [`grip`]: tyre temperature, pressure and wear behind the `GripScale` seam (tyre-thermal.md);
//! - [`suspension`]: the axle model of the suspension hook (suspension.md);
//! - [`aids`], [`steering`] and [`controls`]: the controls hook (aids.md, steering.md);
//! - [`keys`], [`params`], [`presets`] and [`mapping`]: the parameters of a car (parameter-mapping.md);
//! - [`control`], [`model`], [`settings`], [`commands`] and [`setup`]: the switches and the state machine behind them;
//! - this file: the mod itself. It registers the tyre hook only while the master switch is on (and the suspension and controls
//!   hooks only while their units are on too), reads the car's spec when its revision changes, and applies every switch at a
//!   frame boundary.

pub mod aids;
pub mod bridge;
pub mod commands;
pub mod constants;
pub mod control;
mod control_commands;
mod control_hooks;
pub mod controls;
pub mod grip;
pub mod keys;
pub mod mapping;
pub mod model;
pub mod params;
pub mod presets;
pub mod settings;
pub mod setup;
pub mod steering;
pub mod substep;
pub mod suspension;
pub mod thermal;
pub mod tyre_force;
pub mod wheel;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;

use blackbox_modding::abi::BB_EVENT_FRAME_START;
use blackbox_modding::sdk::{Ctx, Error, Event, HandlingHookId, Level, MessageStyle, Mod, ModResult, declare_mod};

use crate::control::{Control, Hook, Step};
use crate::mapping::CarSpec;
use crate::presets::PresetSet;
use crate::setup::{Shared, with};

/// The mod: the state the host's closures share, and the hooks it registered while the model is on.
pub struct AcHandling {
    shared: Shared,
    hooks: [Option<HandlingHookId>; 3],
    spec_error_logged: bool,
}

/// The presets: the shipped files of the mod folder over the embedded copies. The notes are the rejected files.
fn load_presets(ctx: &Ctx) -> ModResult<(PresetSet, Vec<String>)> {
    let loaded = match ctx.mod_dir() {
        Ok(dir) => PresetSet::load(&dir.join("presets")),
        Err(_) => PresetSet::builtin().map(|set| (set, Vec::new())),
    };
    loaded.map_err(|reason| {
        ctx.log(Level::Error, &format!("ac-handling: the built-in presets are broken: {reason}"));
        Error::Host
    })
}

/// The first values of the settings from `config.toml`; the defaults when there is none or it is rejected.
fn load_settings(ctx: &Ctx) -> settings::Settings {
    let Some(config) = ctx.config() else {
        return settings::Settings::default();
    };
    let Ok(text) = config.text() else {
        return settings::Settings::default();
    };
    match settings::parse_config(&text) {
        Ok(values) => values,
        Err(reason) => {
            ctx.log(Level::Warn, &format!("ac-handling: config.toml rejected: {reason}; using the defaults"));
            settings::Settings::default()
        }
    }
}

fn slot(hook: Hook) -> usize {
    match hook {
        Hook::Tyre => 0,
        Hook::Suspension => 1,
        Hook::Controls => 2,
    }
}

fn hook_name(hook: Hook) -> &'static str {
    match hook {
        Hook::Tyre => "tyre",
        Hook::Suspension => "suspension",
        Hook::Controls => "controls",
    }
}

impl AcHandling {
    /// Apply the switches at this frame boundary, read the car when it changed, apply again (the units' defaults are known once
    /// the car is), show the tyre line, and write the queued log lines.
    fn frame(&mut self, ctx: &mut Ctx) {
        self.apply(ctx);
        self.read_spec(ctx);
        self.apply(ctx);
        if let Some(line) = with(&self.shared, |control| control.hud_line()).flatten() {
            self.show(ctx, &line, MessageStyle::Info, 2.0);
        }
        let notes = with(&self.shared, |control| control.take_notes()).unwrap_or_default();
        for note in notes {
            ctx.log(note.level, &note.text);
        }
    }

    /// Do what the plan says: unregister the controls and suspension hooks before the tyre hook, register the tyre hook first.
    fn apply(&mut self, ctx: &mut Ctx) {
        let Some(plan) = with(&self.shared, |control| control.plan()) else {
            return;
        };
        if plan.is_idle() {
            return;
        }
        let step_of = |hook: Hook| match hook {
            Hook::Tyre => plan.tyre,
            Hook::Suspension => plan.suspension,
            Hook::Controls => plan.controls,
        };
        for hook in [Hook::Controls, Hook::Suspension, Hook::Tyre] {
            if step_of(hook) == Step::Unregister {
                self.unregister(ctx, hook);
            }
        }
        for hook in [Hook::Tyre, Hook::Suspension, Hook::Controls] {
            if step_of(hook) == Step::Register {
                self.register(ctx, hook);
            }
        }
    }

    fn register(&mut self, ctx: &mut Ctx, hook: Hook) {
        let Some(handling) = ctx.handling() else {
            with(&self.shared, |control| control.confirm(hook, Step::Register, false));
            return;
        };
        let shared = self.shared.clone();
        let registered = match hook {
            Hook::Tyre => handling.register_tyre(move |_ctx, query| shared.try_borrow_mut().ok()?.tyre(query)),
            Hook::Suspension => {
                handling.register_suspension(move |_ctx, query| shared.try_borrow_mut().ok()?.suspension(query))
            }
            Hook::Controls => {
                handling.register_controls(move |_ctx, query| shared.try_borrow_mut().ok()?.controls(query))
            }
        };
        let name = hook_name(hook);
        match registered {
            Ok(id) => {
                self.hooks[slot(hook)] = Some(id);
                with(&self.shared, |control| control.confirm(hook, Step::Register, true));
                ctx.log(Level::Info, &format!("ac-handling: the {name} hook is on"));
                if hook == Hook::Tyre {
                    self.show(ctx, "AC-style handling: on", MessageStyle::Info, 2.5);
                }
            }
            Err(error) => {
                with(&self.shared, |control| control.confirm(hook, Step::Register, false));
                ctx.log(Level::Warn, &format!("ac-handling: the {name} hook was refused: {error}"));
                if hook == Hook::Tyre {
                    self.show(ctx, "AC-style handling: refused by the game", MessageStyle::Warning, 2.5);
                }
            }
        }
    }

    fn unregister(&mut self, ctx: &mut Ctx, hook: Hook) {
        if let (Some(id), Some(handling)) = (self.hooks[slot(hook)].take(), ctx.handling()) {
            let _ = match hook {
                Hook::Tyre => handling.unregister_tyre(id),
                Hook::Suspension => handling.unregister_suspension(id),
                Hook::Controls => handling.unregister_controls(id),
            };
        }
        with(&self.shared, |control| control.confirm(hook, Step::Unregister, true));
        ctx.log(Level::Info, &format!("ac-handling: the {} hook is off", hook_name(hook)));
        if hook == Hook::Tyre {
            self.show(ctx, "AC-style handling: off", MessageStyle::Info, 2.5);
        }
    }

    /// Read the car's spec when the model needs it (a new car, or none yet) and build its parameter set once.
    fn read_spec(&mut self, ctx: &mut Ctx) {
        if !with(&self.shared, |control| control.wants_spec()).unwrap_or(false) {
            return;
        }
        let Some(vehicles) = ctx.vehicles() else {
            return;
        };
        match vehicles.spec() {
            Ok(spec) => {
                self.spec_error_logged = false;
                with(&self.shared, |control| control.load_spec(CarSpec::from_sdk(&spec)));
            }
            Err(Error::NotReady) => {}
            Err(error) => {
                if !self.spec_error_logged {
                    ctx.log(Level::Warn, &format!("ac-handling: the car's spec was not read: {error}"));
                }
                self.spec_error_logged = true;
            }
        }
    }

    /// A short message on the HUD when the host has one; never an error when it has none.
    fn show(&self, ctx: &Ctx, text: &str, style: MessageStyle, secs: f32) {
        let Some(hud) = ctx.hud() else {
            return;
        };
        let _ = hud.message(text, style, secs);
    }
}

impl Mod for AcHandling {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        ctx.handling().ok_or(Error::Unsupported)?;
        ctx.vehicles().ok_or(Error::Unsupported)?;
        let (presets, rejected) = load_presets(ctx)?;
        for reason in rejected {
            ctx.log(Level::Warn, &format!("ac-handling: {reason}"));
        }
        let first = load_settings(ctx);
        let shared: Shared = Rc::new(RefCell::new(Control::new(presets, ctx.mod_dir().ok(), first)));
        setup::install(ctx, &shared);
        let status = with(&shared, |control| control.status()).unwrap_or_default();
        ctx.log(Level::Info, &format!("ac-handling: loaded ({status})"));
        Ok(AcHandling { shared, hooks: [None; 3], spec_error_logged: false })
    }

    fn event(&mut self, ctx: &mut Ctx, event: &Event<'_>) {
        // By kind, not by variant: a frame-start event the host sent without its payload arrives as `Event::Unknown`, and the
        // switches must still be applied.
        if event.kind() == BB_EVENT_FRAME_START {
            self.frame(ctx);
        }
    }

    fn unload(&mut self, ctx: &mut Ctx) {
        if let Some(handling) = ctx.handling() {
            let (tyre, suspension, controls) = (self.hooks[0].take(), self.hooks[1].take(), self.hooks[2].take());
            let _ = controls.map(|id| handling.unregister_controls(id));
            let _ = suspension.map(|id| handling.unregister_suspension(id));
            let _ = tyre.map(|id| handling.unregister_tyre(id));
        }
        ctx.log(Level::Info, "ac-handling: unloaded");
    }
}

declare_mod!(AcHandling);
