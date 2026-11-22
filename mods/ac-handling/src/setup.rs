//! The player's switches on the host: the `bb.settings` master toggle and preset list, the `bb.menus` grip slider, the
//! `bb.input` hotkey and the `bb.commands` console command. All of them are optional interfaces: one that is absent (or
//! refuses) is logged and the others still work. None of them touches the tyre hook: each only changes the request held by
//! [`Control`], and the frame start applies it (see `lib.rs`).

use std::cell::RefCell;
use std::rc::Rc;

use blackbox_modding::sdk::{Ctx, Error, Level, SettingValue, SliderRange};

use crate::aids::{MAX_CLICKS, MAX_ESC_LEVEL, MAX_LEVEL};
use crate::commands::LevelKind;
use crate::commands::{self, HELP};
use crate::control::Control;
use crate::settings::{GRIP_GAIN_MAX, GRIP_GAIN_MIN, PresetChoice, Unit, UnitMode};
use crate::steering::Device;

/// The state the closures of the host share. The host is single threaded: nothing here crosses a thread.
pub type Shared = Rc<RefCell<Control>>;

/// The id of the master toggle in `bb.settings`.
pub const OPTION_ENABLED: &str = "ac_handling_enabled";
/// The id of the preset list in `bb.settings`.
pub const OPTION_PRESET: &str = "ac_handling_preset";
/// The name of the grip slider in `bb.menus`.
pub const MENU_GRIP: &str = "ac_handling_grip";
/// The id of the HUD toggle in `bb.settings`.
pub const OPTION_HUD: &str = "ac_handling_hud";
/// The id of the steering device list in `bb.settings`.
pub const OPTION_DEVICE: &str = "ac_handling_device";
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
        with(&toggle, |control| control.set_requested(on));
    });
    if let Err(error) = added {
        warn(ctx, "the master toggle", error);
    }
    if settings.get(OPTION_ENABLED).is_ok_and(|value| value == "on") {
        with(shared, |control| control.set_requested(true));
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
    unit_options(ctx, &settings, shared);
}

/// The four unit lists (`Preset default`, `On`, `Off`), the tyre HUD toggle and the steering device list.
fn unit_options(ctx: &Ctx, settings: &blackbox_modding::sdk::Settings, shared: &Shared) {
    let section = settings.section(SECTION);
    let mode_values = [
        SettingValue { id: "preset", label: "Preset default" },
        SettingValue { id: "on", label: "On" },
        SettingValue { id: "off", label: "Off" },
    ];
    let rows = [
        (Unit::Suspension, "ac_handling_suspension", "AC-style suspension"),
        (Unit::Aids, "ac_handling_aids", "AC-style driver aids"),
        (Unit::Steering, "ac_handling_steering", "AC-style steering"),
        (Unit::Thermal, "ac_handling_thermal", "AC-style tyre temperature and wear"),
    ];
    for (unit, id, label) in rows {
        let handler = shared.clone();
        let added = section.add_option_with(id, label, &mode_values, move |_ctx, value| {
            let Some(mode) = UnitMode::parse(value) else {
                return;
            };
            with(&handler, |control| control.set_unit(unit, mode));
        });
        if let Err(error) = added {
            warn(ctx, label, error);
            continue;
        }
        let saved = settings.get(id).ok().and_then(|value| UnitMode::parse(&value));
        if let Some(mode) = saved.filter(|mode| *mode != UnitMode::Preset) {
            with(shared, |control| control.set_unit(unit, mode));
        }
    }
    let first_hud = with(shared, |control| control.hud).unwrap_or(false);
    let hud = shared.clone();
    let added = section.add_toggle(OPTION_HUD, "AC-style tyre temperatures on the HUD", first_hud, move |_ctx, on| {
        with(&hud, |control| control.hud = on);
    });
    if let Err(error) = added {
        warn(ctx, "the tyre HUD toggle", error);
    } else if settings.get(OPTION_HUD).is_ok_and(|value| value == "on") {
        with(shared, |control| control.hud = true);
    }
    let devices =
        [SettingValue { id: "pad", label: "Pad or keyboard" }, SettingValue { id: "wheel", label: "Steering wheel" }];
    let device = shared.clone();
    let added = section.add_option_with(OPTION_DEVICE, "AC-style steering device", &devices, move |_ctx, value| {
        let Some(choice) = Device::parse(value) else {
            return;
        };
        with(&device, |control| control.device = choice);
    });
    if let Err(error) = added {
        warn(ctx, "the steering device list", error);
        return;
    }
    let saved = settings.get(OPTION_DEVICE).ok().and_then(|value| Device::parse(&value));
    if let Some(choice) = saved.filter(|choice| *choice != Device::Pad) {
        with(shared, |control| control.device = choice);
    }
}

/// One slider row of the menu: name, label, range, default and what a change does.
type MenuRow = (&'static str, &'static str, SliderRange, f32, fn(&mut Control, f64));

/// One slider row: its saved value (or the default) comes back through `apply` once, then each change goes through `change`.
fn slider(
    ctx: &Ctx,
    menus: &blackbox_modding::sdk::Menus,
    shared: &Shared,
    row: (&str, &str, SliderRange, f32),
    apply: fn(&mut Control, f64),
) {
    let (name, label, range, default) = row;
    let handler = shared.clone();
    let added = menus.slider(name, label, range, default, move |_ctx, value| {
        with(&handler, |control| apply(control, f64::from(value)));
    });
    match added {
        Ok(id) => {
            let saved = menus.get(id).unwrap_or(default);
            if (saved - default).abs() > f32::EPSILON {
                with(shared, |control| apply(control, f64::from(saved)));
            }
        }
        Err(error) => warn(ctx, name, error),
    }
}

fn menus(ctx: &Ctx, shared: &Shared) {
    let Some(menus) = ctx.menus() else {
        return;
    };
    let gain = with(shared, |control| control.grip_gain).unwrap_or(1.0);
    let grip = SliderRange { min: GRIP_GAIN_MIN as f32, max: GRIP_GAIN_MAX as f32, step: 0.05 };
    let rows: [MenuRow; 8] = [
        (MENU_GRIP, "AC-style handling: grip scale", grip, gain as f32, Control::set_grip_gain),
        (
            "ac_handling_tc",
            "AC-style aids: traction control (0 off)",
            SliderRange { min: 0.0, max: MAX_LEVEL as f32, step: 1.0 },
            0.0,
            |control, value| control.set_level(LevelKind::Tc, value as i32),
        ),
        (
            "ac_handling_abs",
            "AC-style aids: ABS (0 off)",
            SliderRange { min: 0.0, max: MAX_LEVEL as f32, step: 1.0 },
            0.0,
            |control, value| control.set_level(LevelKind::Abs, value as i32),
        ),
        (
            "ac_handling_esc",
            "AC-style aids: stability control (0 off)",
            SliderRange { min: 0.0, max: MAX_ESC_LEVEL as f32, step: 1.0 },
            0.0,
            |control, value| control.set_level(LevelKind::Esc, value as i32),
        ),
        (
            "ac_handling_bias",
            "AC-style aids: brake bias (clicks)",
            SliderRange { min: -(MAX_CLICKS as f32), max: MAX_CLICKS as f32, step: 1.0 },
            0.0,
            |control, value| control.set_bias(value as i32),
        ),
        (
            "ac_handling_steer_sens",
            "AC-style steering: speed sensitivity (pad)",
            SliderRange { min: 0.0, max: 1.0, step: 0.05 },
            1.0,
            |control, value| control.steer_sens = Some(value),
        ),
        (
            "ac_handling_ffb_gain",
            "AC-style steering: force feedback gain",
            SliderRange { min: 0.0, max: 3.0, step: 0.1 },
            1.0,
            |control, value| control.ffb_gain = Some(value),
        ),
        (
            "ac_handling_ffb_curve",
            "AC-style steering: force feedback curve",
            SliderRange { min: 0.4, max: 1.5, step: 0.05 },
            1.0,
            |control, value| control.ffb_curve = Some(value),
        ),
    ];
    for (name, label, range, default, apply) in rows {
        slider(ctx, &menus, shared, (name, label, range, default), apply);
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
