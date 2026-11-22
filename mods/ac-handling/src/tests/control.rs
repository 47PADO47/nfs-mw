//! The switches without a host: default off, the master toggle applied at a frame boundary, the tyre hook declining every
//! role but the player's, the parameter set built once per car change, the car id and the tuning file, and the console command.
//! The same logic runs against the real loader in `tests/real_loader/`.

use blackbox_modding::sdk::{Level, Role, TyreQuery};

use super::TempDir;
use crate::commands::{self, Command};
use crate::control::{Control, Hook, Plan, Step};
use crate::mapping::CarSpec;
use crate::presets::{Compound, PresetSet};
use crate::settings::{PresetChoice, Settings};

pub fn control(settings: Settings) -> Control {
    Control::new(PresetSet::builtin().expect("the shipped presets"), None, settings)
}

pub fn query(wheel: u32) -> TyreQuery {
    TyreQuery {
        wheel_index: wheel,
        role: Some(Role::Player),
        loaded: true,
        vehicle_revision: 1,
        dt: 1.0 / 60.0,
        vertical_load_n: 3700.0,
        patch_forward_velocity: 20.0,
        patch_lateral_velocity: 2.0,
        wheel_spin: 20.0 / 0.34,
        wheel_radius_m: 0.34,
        wheel_inertia_kgm2: 10.0,
        corner_mass_kg: 362.0,
        surface_lateral: 1.0,
        surface_drive: 1.0,
        surface_rolling: 1.0,
        tyre_condition: 1.0,
        ambient_temperature_c: 20.0,
        road_temperature_c: 25.0,
        body_speed: 20.0,
        ..TyreQuery::default()
    }
}

/// Do every step of a plan as a host that accepts everything would.
pub fn accept(control: &mut Control) -> Plan {
    let plan = control.plan();
    for (hook, step) in [(Hook::Tyre, plan.tyre), (Hook::Suspension, plan.suspension), (Hook::Controls, plan.controls)]
    {
        control.confirm(hook, step, true);
    }
    plan
}

/// A control that is on, has its hooks registered, and has read the reference car.
pub fn running_with(settings: Settings) -> Control {
    let mut control = control(Settings { enabled: true, ..settings });
    assert_eq!(control.plan().tyre, Step::Register);
    accept(&mut control);
    control.load_spec(CarSpec::ref_sport());
    accept(&mut control);
    control
}

pub fn running() -> Control {
    running_with(Settings::default())
}

#[test]
fn the_model_is_off_by_default_and_registers_nothing() {
    let mut control = control(Settings::default());
    assert!(!control.requested && control.applied == Default::default());
    assert!(control.plan().is_idle(), "master off: no hook at all");
    assert!(!control.wants_spec(), "no spec is read while the model is off");
    assert!(control.tyre(&query(0)).is_none());
}

#[test]
fn switching_on_registers_at_the_frame_boundary_and_reads_the_spec_once() {
    let mut control = control(Settings::default());
    control.run(Command::On);
    assert_eq!(control.plan().tyre, Step::Register, "the request is applied by the next plan, not inside the command");
    assert!(!control.applied.tyre);
    accept(&mut control);
    assert!(control.applied.tyre && control.wants_spec());
    assert!(control.tyre(&query(0)).is_none(), "no parameter set yet: the built-in tyre runs");
    control.load_spec(CarSpec::ref_sport());
    assert!(!control.wants_spec(), "the spec is read once, not every frame");
    assert!(control.tyre(&query(0)).is_some());
}

#[test]
fn switching_off_unregisters_and_drops_the_model_so_switching_on_again_starts_cold() {
    let mut control = running();
    for _ in 0..30 {
        control.tyre(&query(2));
    }
    control.run(Command::Off);
    let plan = control.plan();
    assert_eq!(plan.tyre, Step::Unregister);
    assert_eq!(plan.suspension, Step::Unregister, "master off unregisters every hook");
    accept(&mut control);
    assert!(control.applied == Default::default());
    assert!(control.tyre(&query(0)).is_none(), "off means the built-in tyre");
    control.run(Command::On);
    accept(&mut control);
    assert!(control.wants_spec(), "the model is rebuilt from the spec, with fresh wheels");
    control.load_spec(CarSpec::ref_sport());
    assert!(control.tyre(&query(2)).is_some());
}

#[test]
fn a_refused_registration_clears_the_request_so_it_is_not_retried_every_frame() {
    let mut control = control(Settings { enabled: true, ..Settings::default() });
    assert_eq!(control.plan().tyre, Step::Register);
    control.confirm(Hook::Tyre, Step::Register, false);
    assert!(!control.requested && !control.applied.tyre);
    assert!(control.plan().is_idle());
}

#[test]
fn the_hotkey_flips_the_request() {
    let mut control = control(Settings::default());
    control.toggle();
    assert!(control.requested);
    control.toggle();
    assert!(!control.requested);
}

#[test]
fn only_the_player_car_is_handled() {
    let mut control = running();
    for role in [Role::Traffic, Role::Cop, Role::Racer] {
        assert!(control.tyre(&TyreQuery { role: Some(role), ..query(0) }).is_none(), "{role:?}");
    }
    assert!(control.tyre(&TyreQuery { role: None, ..query(0) }).is_none(), "an unknown role");
    assert!(control.tyre(&query(0)).is_some());
}

#[test]
fn a_query_the_model_may_not_answer_is_declined() {
    let mut control = running();
    assert!(control.tyre(&TyreQuery { corner_mass_kg: 0.0, ..query(0) }).is_none());
    assert!(control.tyre(&TyreQuery { wheel_spin: f32::NAN, ..query(0) }).is_none());
    assert!(control.tyre(&TyreQuery { dt: 0.0, ..query(0) }).is_none());
    assert!(control.tyre(&TyreQuery { vertical_load_n: -1.0, ..query(0) }).is_none());
    assert!(control.tyre(&TyreQuery { camber_rad: f32::NAN, ..query(0) }).is_some(), "a NaN camber counts as none");
    assert!(control.tyre(&query(4)).is_none(), "a wheel index past 3");
    assert!(control.tyre(&query(0)).is_some(), "and the wheel recovers on the next good call");
}

#[test]
fn an_answer_is_always_finite_and_in_the_hosts_range() {
    let mut control = running();
    for step in 0..400_u32 {
        let wild = TyreQuery {
            vertical_load_n: (step % 13) as f32 * 1500.0,
            loaded: step % 5 != 0,
            patch_forward_velocity: (step as f32 - 200.0) * 0.7,
            patch_lateral_velocity: (step as f32 * 0.37).sin() * 40.0,
            wheel_spin: (step as f32 - 150.0) * 1.5,
            drive_torque_nm: (step as f32 - 200.0) * 100.0,
            brake_torque_nm: (step % 11) as f32 * 900.0,
            camber_rad: ((step % 7) as f32 - 3.0) * 0.02,
            ..query(step % 4)
        };
        let Some(answer) = control.tyre(&wild) else {
            continue;
        };
        let numbers = [
            answer.longitudinal_force_n,
            answer.lateral_force_n,
            answer.aligning_torque_nm,
            answer.wheel_spin,
            answer.traction,
            answer.slip,
            answer.slip_angle,
        ];
        assert!(numbers.iter().all(|value| value.is_finite()), "{answer:?}");
        assert!(answer.longitudinal_force_n.abs() <= 1.0e6 && answer.wheel_spin.abs() <= 5000.0);
        assert!((0.0..=1.0).contains(&answer.traction) && answer.slip_angle.abs() <= 1.0);
    }
}

#[test]
fn an_airborne_wheel_answers_zero_forces_and_a_spin() {
    let mut control = running();
    let air = TyreQuery {
        loaded: false,
        vertical_load_n: 0.0,
        patch_forward_velocity: 0.0,
        patch_lateral_velocity: 0.0,
        ..query(1)
    };
    let answer = control.tyre(&air).expect("an airborne wheel is handled");
    assert_eq!((answer.longitudinal_force_n, answer.lateral_force_n, answer.traction), (0.0, 0.0, 0.0));
    assert!(answer.wheel_spin > 0.0);
}

#[test]
fn a_new_revision_keeps_the_old_parameters_for_the_tick_and_asks_for_the_spec() {
    let mut control = running();
    assert!(!control.wants_spec());
    let answer = control.tyre(&TyreQuery { vehicle_revision: 2, ..query(0) });
    assert!(answer.is_some(), "the old parameters answer this tick");
    assert!(control.wants_spec(), "the glue reads the spec at the next frame start");
    control.load_spec(CarSpec { revision: 2, ..CarSpec::ref_sport() });
    assert!(!control.wants_spec());
    assert!(control.tyre(&TyreQuery { vehicle_revision: 2, ..query(0) }).is_some());
    assert!(!control.wants_spec(), "the revision matches: no read per tick");
}

#[test]
fn the_grip_gain_scales_the_lateral_force() {
    let mut control = running();
    let full = control.tyre(&query(0)).expect("handled").lateral_force_n.abs();
    let mut weaker = running();
    weaker.set_grip_gain(0.5);
    let half = weaker.tyre(&query(0)).expect("handled").lateral_force_n.abs();
    assert!(half < full && half > 0.3 * full, "{half} against {full}");
    weaker.set_grip_gain(99.0);
    assert_eq!(weaker.grip_gain, 1.5, "the gain is kept inside its range");
    weaker.set_grip_gain(f64::NAN);
    assert_eq!(weaker.grip_gain, 1.0);
}

#[test]
fn the_preset_setting_rebuilds_the_parameters_of_the_current_car() {
    let mut control = running();
    let before = control.status();
    assert!(before.contains("sport/sport"), "{before}");
    control.run(Command::Preset(PresetChoice::Forced(Compound::Slick)));
    let after = control.status();
    assert!(after.contains("slick/sport") && after.contains("preset slick"), "{after}");
    control.run(Command::Preset(PresetChoice::Auto));
    assert!(control.status().contains("sport/sport"));
}

#[test]
fn a_car_is_logged_once_and_a_tuning_file_is_read_from_the_mod_folder() {
    let dir = TempDir::new("car-file");
    let name = CarSpec::ref_sport().key_name();
    assert_eq!(name, "spec-1450-245-300");
    let make = || Control::new(PresetSet::builtin().expect("presets"), Some(dir.0.clone()), Settings::default());
    let mut control = make();
    control.load_spec(CarSpec::ref_sport());
    control.load_spec(CarSpec::ref_sport());
    let notes = control.take_notes();
    assert_eq!(notes.len(), 1, "an unknown car is logged once: {notes:?}");
    assert!(notes[0].text.contains("no tuning/spec-1450-245-300.toml") && notes[0].text.contains("sport/sport"));
    assert!(control.take_notes().is_empty());

    dir.write("tuning/spec-1450-245-300.toml", "schema = 1\ncompound = \"road\"\n");
    let mut control = make();
    control.load_spec(CarSpec::ref_sport());
    assert!(control.status().contains("road/sport"), "{}", control.status());
    assert!(control.take_notes()[0].text.contains("tuning/spec-1450-245-300.toml"));

    // A file in the old `cars/` folder is not read: the host reads that folder as car manifests.
    let other = TempDir::new("cars-folder");
    other.write("cars/spec-1450-245-300.toml", "schema = 1\ncompound = \"road\"\n");
    let mut control = Control::new(PresetSet::builtin().expect("presets"), Some(other.0.clone()), Settings::default());
    control.load_spec(CarSpec::ref_sport());
    assert!(control.status().contains("sport/sport"));

    dir.write("tuning/spec-1450-245-300.toml", "schema = 1\nmystery = 1\n");
    let mut control = make();
    control.load_spec(CarSpec::ref_sport());
    let notes = control.take_notes();
    assert_eq!(notes[0].level, Level::Warn, "{notes:?}");
    assert!(notes[0].text.contains("mystery") && notes[0].text.contains("automatic mapping"));
    assert!(control.status().contains("sport/sport"), "a rejected tuning file leaves the automatic mapping");
}

#[test]
fn an_inertia_scale_below_the_stable_range_warns_once() {
    let dir = TempDir::new("light-wheel");
    dir.write("tuning/spec-1450-245-300.toml", "schema = 1\n[tyre]\ninertia_scale = 0.3\n");
    let mut control = Control::new(PresetSet::builtin().expect("presets"), Some(dir.0.clone()), Settings::default());
    control.load_spec(CarSpec::ref_sport());
    control.load_spec(CarSpec::ref_sport());
    let warnings: Vec<_> = control.take_notes().into_iter().filter(|note| note.level == Level::Warn).collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].text.contains("inertia_scale 0.30"));
}

#[test]
fn the_console_command_words() {
    assert_eq!(commands::parse(&["on"]), Ok(Command::On));
    assert_eq!(commands::parse(&["off"]), Ok(Command::Off));
    assert_eq!(commands::parse(&["status"]), Ok(Command::Status));
    assert_eq!(commands::parse(&["preset", "road"]), Ok(Command::Preset(PresetChoice::Forced(Compound::Road))));
    assert_eq!(commands::parse(&["preset", "auto"]), Ok(Command::Preset(PresetChoice::Auto)));
    let bad_lines: [&[&str]; 6] = [&[], &["preset"], &["preset", "gold"], &["on", "off"], &["maybe"], &["tc", "9"]];
    for bad in bad_lines {
        let usage = commands::parse(bad).expect_err("usage");
        assert!(usage.contains("ac-handling on|off|status"), "{usage}");
    }
    let mut control = control(Settings::default());
    assert!(control.run(Command::Status).contains("off"));
    assert!(control.run(Command::On).contains("on"));
    assert!(control.status().contains("on, hook not registered"));
}
