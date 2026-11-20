//! `ac-handling`: an Assetto Corsa-style handling mod for the player's car of NFS: Most Wanted.
//!
//! It targets the *feel* of Assetto Corsa with a model written from our own specs (`docs/specs/ac-handling/`): a slip curve
//! with a peak and a fall-off, load sensitivity, one grip budget for both directions, and a wheel that is integrated in
//! substeps inside the tyre hook (`bb.handling`). It is an independent project, not affiliated with or endorsed by Kunos
//! Simulazioni or 505 Games, and holds no game file and no foreign code.
//!
//! The crate is a set of pure modules and a thin glue:
//!
//! - [`tyre_force`], [`wheel`] and [`substep`]: the model (tyre-force.md and wheel-dynamics.md);
//! - [`keys`], [`params`], [`presets`] and [`mapping`]: the parameters of a car (parameter-mapping.md);
//! - [`grip`]: the seam where the thermal model plugs in (stage 5);
//! - [`control`], [`settings`], [`commands`] and [`setup`]: the switches and the state machine behind them;
//! - this file: the mod itself. It registers the tyre hook only while the master switch is on, reads the car's spec when
//!   its revision changes, and applies every switch at a frame boundary.

pub mod bridge;
pub mod commands;
pub mod constants;
pub mod control;
pub mod grip;
pub mod keys;
pub mod mapping;
pub mod params;
pub mod presets;
pub mod settings;
pub mod setup;
pub mod substep;
pub mod tyre_force;
pub mod wheel;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;

use blackbox_modding::abi::BB_EVENT_FRAME_START;
use blackbox_modding::sdk::{Ctx, Error, Event, HandlingHookId, Level, MessageStyle, Mod, ModResult, declare_mod};

use crate::control::{Control, Plan};
use crate::mapping::CarSpec;
use crate::presets::PresetSet;
use crate::setup::{Shared, with};

/// The mod: the state the host's closures share, and the hook it registered while the model is on.
pub struct AcHandling {
    shared: Shared,
    hook: Option<HandlingHookId>,
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

impl AcHandling {
    /// Apply the switches at this frame boundary, read the car when it changed, and write the queued log lines.
    fn frame(&mut self, ctx: &mut Ctx) {
        let plan = with(&self.shared, |control| control.plan()).unwrap_or(Plan::Nothing);
        match plan {
            Plan::Register => self.register(ctx),
            Plan::Unregister => self.unregister(ctx),
            Plan::Nothing => {}
        }
        self.read_spec(ctx);
        let notes = with(&self.shared, |control| control.take_notes()).unwrap_or_default();
        for note in notes {
            ctx.log(note.level, &note.text);
        }
    }

    fn register(&mut self, ctx: &mut Ctx) {
        let Some(handling) = ctx.handling() else {
            with(&self.shared, |control| control.confirm(Plan::Register, false));
            return;
        };
        let shared = self.shared.clone();
        let registered = handling.register_tyre(move |_ctx, query| shared.try_borrow_mut().ok()?.tyre(query));
        match registered {
            Ok(id) => {
                self.hook = Some(id);
                with(&self.shared, |control| control.confirm(Plan::Register, true));
                self.say(ctx, "AC-style handling: on", MessageStyle::Info);
                ctx.log(Level::Info, "ac-handling: the tyre hook is on");
            }
            Err(error) => {
                with(&self.shared, |control| control.confirm(Plan::Register, false));
                self.say(ctx, "AC-style handling: refused by the game", MessageStyle::Warning);
                ctx.log(Level::Warn, &format!("ac-handling: the tyre hook was refused: {error}"));
            }
        }
    }

    fn unregister(&mut self, ctx: &mut Ctx) {
        if let (Some(id), Some(handling)) = (self.hook.take(), ctx.handling()) {
            let _ = handling.unregister_tyre(id);
        }
        with(&self.shared, |control| control.confirm(Plan::Unregister, true));
        self.say(ctx, "AC-style handling: off", MessageStyle::Info);
        ctx.log(Level::Info, "ac-handling: the tyre hook is off");
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

    /// A short message on the HUD when the host has one.
    fn say(&self, ctx: &Ctx, text: &str, style: MessageStyle) {
        let Some(hud) = ctx.hud() else {
            return;
        };
        let _ = hud.message(text, style, 2.5);
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
        Ok(AcHandling { shared, hook: None, spec_error_logged: false })
    }

    fn event(&mut self, ctx: &mut Ctx, event: &Event<'_>) {
        // By kind, not by variant: a frame-start event the host sent without its payload arrives as `Event::Unknown`, and the
        // switches must still be applied.
        if event.kind() == BB_EVENT_FRAME_START {
            self.frame(ctx);
        }
    }

    fn unload(&mut self, ctx: &mut Ctx) {
        if let (Some(id), Some(handling)) = (self.hook.take(), ctx.handling()) {
            let _ = handling.unregister_tyre(id);
        }
        ctx.log(Level::Info, "ac-handling: unloaded");
    }
}

declare_mod!(AcHandling);
