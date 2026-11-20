//! The player's switches on the host: the `bb.settings` master toggle and preset list, the `bb.menus` grip slider, the
//! `bb.input` hotkey and the `bb.commands` console command. All of them are optional interfaces: one that is absent (or
//! refuses) is logged and the others still work. None of them touches the tyre hook: each only changes the request held by
//! [`Control`], and the frame start applies it (see `lib.rs`).

use std::cell::RefCell;
use std::rc::Rc;

use blackbox_modding::sdk::{Ctx, Error, Level, SettingValue, SliderRange};

use crate::commands::{self, HELP};
use crate::control::Control;
use crate::settings::{GRIP_GAIN_MAX, GRIP_GAIN_MIN, PresetChoice};

/// The state the closures of the host share. The host is single threaded: nothing here crosses a thread.
pub type Shared = Rc<RefCell<Control>>;

/// The id of the master toggle in `bb.settings`.
pub const OPTION_ENABLED: &str = "ac_handling_enabled";
/// The id of the preset list in `bb.settings`.
pub const OPTION_PRESET: &str = "ac_handling_preset";
/// The name of the grip slider in `bb.menus`.
pub const MENU_GRIP: &str = "ac_handling_grip";
/// The name of the hotkey action.
pub const HOTKEY: &str = "ac_handling_toggle";
/// The default binding of the hotkey, in the form of the `bind` command.
pub const HOTKEY_BINDING: &str = "key:F1";
/// The console command.
pub const COMMAND: &str = "ac-handling";
/// The section of the gameplay settings that holds the mod's options.
pub const SECTION: &str = "AC-style handling";

/// Run `f` on the control unless it is borrowed (a re-entrant call): then nothing happens.
pub fn with<R>(shared: &Shared, f: impl FnOnce(&mut Control) -> R) -> Option<R> {
    let mut control = shared.try_borrow_mut().ok()?;
    Some(f(&mut control))
}

/// Add every switch the host offers.
pub fn install(ctx: &Ctx, shared: &Shared) {
    settings(ctx, shared);
    menus(ctx, shared);
    hotkey(ctx, shared);
    command(ctx, shared);
}

fn warn(ctx: &Ctx, what: &str, error: Error) {
    ctx.log(Level::Warn, &format!("ac-handling: {what} not added: {error}"));
}

fn settings(ctx: &Ctx, shared: &Shared) {
    let Some(settings) = ctx.settings() else {
        ctx.log(Level::Info, "ac-handling: bb.settings is absent; use the hotkey or the console command");
        return;
    };
    let section = settings.section(SECTION);
    let first = with(shared, |control| (control.requested, control.preset)).unwrap_or_default();
    let toggle = shared.clone();
    let added = section.add_toggle(OPTION_ENABLED, "AC-style handling", first.0, move |_ctx, on| {
        with(&toggle, |control| control.requested = on);
    });
    if let Err(error) = added {
        warn(ctx, "the master toggle", error);
    }
    if settings.get(OPTION_ENABLED).is_ok_and(|value| value == "on") {
        with(shared, |control| control.requested = true);
    }
    let labels = ["Automatic", "Economy", "Road", "Sport", "Semi-slick", "Slick"];
    let values: Vec<SettingValue<'_>> =
        PresetChoice::NAMES.iter().zip(labels).map(|(id, label)| SettingValue { id, label }).collect();
    let list = shared.clone();
    let added = section.add_option_with(OPTION_PRESET, "AC-style tyre preset", &values, move |_ctx, value| {
        let Some(choice) = PresetChoice::parse(value) else {
            return;
        };
        with(&list, |control| control.set_preset(choice));
    });
    if let Err(error) = added {
        warn(ctx, "the preset list", error);
        return;
    }
    // A saved choice other than the default wins over `config.toml`; "auto" is what an untouched list reads, so it leaves the
    // config's preset alone.
    let chosen = settings.get(OPTION_PRESET).ok().and_then(|value| PresetChoice::parse(&value));
    if let Some(choice) = chosen.filter(|choice| *choice != PresetChoice::Auto) {
        with(shared, |control| control.set_preset(choice));
    }
}

fn menus(ctx: &Ctx, shared: &Shared) {
    let Some(menus) = ctx.menus() else {
        return;
    };
    let gain = with(shared, |control| control.grip_gain).unwrap_or(1.0);
    let slider = shared.clone();
    let range = SliderRange { min: GRIP_GAIN_MIN as f32, max: GRIP_GAIN_MAX as f32, step: 0.05 };
    let added = menus.slider(MENU_GRIP, "AC-style handling: grip scale", range, gain as f32, move |_ctx, value| {
        with(&slider, |control| control.set_grip_gain(f64::from(value)));
    });
    match added {
        Ok(id) => {
            if let Ok(saved) = menus.get(id) {
                with(shared, |control| control.set_grip_gain(f64::from(saved)));
            }
        }
        Err(error) => warn(ctx, "the grip slider", error),
    }
}

fn hotkey(ctx: &Ctx, shared: &Shared) {
    let Some(input) = ctx.input() else {
        return;
    };
    let key = shared.clone();
    let added = input.register(HOTKEY, "Toggle AC-style handling", HOTKEY_BINDING, move |_ctx| {
        with(&key, |control| control.toggle());
    });
    if let Err(error) = added {
        warn(ctx, "the hotkey", error);
    }
}

fn command(ctx: &Ctx, shared: &Shared) {
    let Some(console) = ctx.commands() else {
        return;
    };
    let run = shared.clone();
    let added = console.register(COMMAND, HELP, move |_ctx, args| {
        let command = match commands::parse(args) {
            Ok(command) => command,
            Err(usage) => return Ok(usage),
        };
        Ok(with(&run, |control| control.run(command)).unwrap_or_else(|| "ac-handling: busy, try again".to_owned()))
    });
    if let Err(error) = added {
        warn(ctx, "the console command", error);
    }
}
