//! The optional units: each one toggles its hook independently, master off registers nothing, every hook declines every role but the
//! player's and every invalid value, the thermal factor and the camber reach the tyre, the aids and the steering answer through the
//! controls hook, and the commands and defaults that drive them.

use blackbox_modding::sdk::{ControlsQuery, Role, SuspensionQuery, SuspensionWheelQuery, TyreQuery};

use super::control::{accept, query, running, running_with};
use crate::commands::{self, Command, LevelKind};
use crate::control::{Hook, Step};
use crate::mapping::CarSpec;
use crate::settings::{Settings, Unit, UnitMode, UnitModes, parse_config};

const DT: f32 = 1.0 / 60.0;

fn susp_query(axle: u32) -> SuspensionQuery {
    let wheel = SuspensionWheelQuery {
        loaded: true,
        compression_m: 0.026,
        static_load_n: 3740.0,
        ..SuspensionWheelQuery::default()
    };
    SuspensionQuery {
        axle,
        role: Some(Role::Player),
        vehicle_revision: 1,
        dt: DT,
        rest_travel_m: 0.026,
        max_travel_m: 0.15,
        corner_mass_kg: 362.0,
        wheels: [wheel, wheel],
    }
}

fn ctl_query() -> ControlsQuery {
    ControlsQuery {
        role: Some(Role::Player),
        vehicle_revision: 1,
        dt: DT,
        throttle: 1.0,
        brake: 0.5,
        forward_speed: 20.0,
        wheel_loaded: [true; 4],
        ..ControlsQuery::default()
    }
}

fn modes(suspension: UnitMode, aids: UnitMode, steering: UnitMode, thermal: UnitMode) -> Settings {
    Settings { units: UnitModes { suspension, aids, steering, thermal }, ..Settings::default() }
}

#[test]
fn master_off_registers_nothing_whatever_the_units_say() {
    let on = modes(UnitMode::On, UnitMode::On, UnitMode::On, UnitMode::On);
    let mut control = super::control::control(on);
    assert!(control.plan().is_idle());
    control.set_unit(Unit::Aids, UnitMode::Off);
    control.set_unit(Unit::Aids, UnitMode::On);
    assert!(control.plan().is_idle(), "unit switches alone never register a hook");
}

#[test]
fn each_unit_toggles_its_own_hook_and_nothing_else() {
    // Defaults of the sport preset: suspension and thermal on, aids and steering off.
    let mut control = running();
    assert!(control.applied.tyre && control.applied.suspension && !control.applied.controls);
    control.set_unit(Unit::Suspension, UnitMode::Off);
    let plan = control.plan();
    assert_eq!((plan.tyre, plan.suspension, plan.controls), (Step::Nothing, Step::Unregister, Step::Nothing));
    accept(&mut control);
    control.set_unit(Unit::Aids, UnitMode::On);
    let plan = control.plan();
    assert_eq!((plan.tyre, plan.suspension, plan.controls), (Step::Nothing, Step::Nothing, Step::Register));
    accept(&mut control);
    control.set_unit(Unit::Steering, UnitMode::On);
    assert!(control.plan().is_idle(), "the controls hook is already in place for the steering part");
    control.set_unit(Unit::Aids, UnitMode::Off);
    assert!(control.plan().is_idle(), "steering keeps the controls hook");
    control.set_unit(Unit::Steering, UnitMode::Off);
    assert_eq!(control.plan().controls, Step::Unregister);
    accept(&mut control);
    control.set_unit(Unit::Suspension, UnitMode::Preset);
    assert_eq!(control.plan().suspension, Step::Register, "back to the preset's default, which is on");
    accept(&mut control);
    control.set_unit(Unit::Thermal, UnitMode::Off);
    assert!(control.plan().is_idle(), "thermal has no hook of its own");
    assert!(control.applied.tyre && control.applied.suspension && !control.applied.controls);
}

#[test]
fn a_refused_unit_hook_is_blocked_until_a_switch_changes() {
    let mut control = running_with(modes(UnitMode::Off, UnitMode::On, UnitMode::Off, UnitMode::Preset));
    assert!(control.applied.controls);
    control.set_unit(Unit::Suspension, UnitMode::On);
    assert_eq!(control.plan().suspension, Step::Register);
    control.confirm(Hook::Suspension, Step::Register, false);
    assert!(control.plan().is_idle(), "refused once, not retried every frame");
    assert!(control.applied.tyre, "a refused suspension hook does not touch the tyre hook");
    control.set_unit(Unit::Thermal, UnitMode::Off);
    assert_eq!(control.plan().suspension, Step::Register, "a changed switch tries again");
}

#[test]
fn the_preset_decides_the_default_of_a_unit_the_player_left_alone() {
    let control = running();
    let defaults = control.unit_defaults();
    assert!(defaults.suspension && defaults.thermal && !defaults.aids && !defaults.steering);
    assert!(control.unit_on(Unit::Suspension) && !control.unit_on(Unit::Aids));
    let forced_off = running_with(modes(UnitMode::Off, UnitMode::Preset, UnitMode::Preset, UnitMode::Off));
    assert!(!forced_off.unit_on(Unit::Suspension) && !forced_off.applied.suspension);
    let settings =
        parse_config("suspension = \"off\"\naids = \"on\"\ntc = 3\nabs = 2\nesc = 1\nbrake_bias_clicks = -4\n")
            .expect("valid");
    assert_eq!(settings.units.suspension, UnitMode::Off);
    assert_eq!(settings.levels.over(Default::default()).bias_clicks, -4);
    assert!(
        parse_config("tc = 9").is_err()
            && parse_config("suspension = \"maybe\"").is_err()
            && parse_config("brake_bias_clicks = 99").is_err()
    );
}

#[test]
fn every_hook_answers_the_player_only() {
    let mut control = running_with(modes(UnitMode::On, UnitMode::On, UnitMode::On, UnitMode::On));
    assert!(control.suspension(&susp_query(0)).is_some() && control.suspension(&susp_query(1)).is_some());
    assert!(control.controls(&ctl_query()).is_some());
    for role in [Role::Traffic, Role::Cop, Role::Racer] {
        assert!(control.tyre(&TyreQuery { role: Some(role), ..query(0) }).is_none());
        assert!(control.suspension(&SuspensionQuery { role: Some(role), ..susp_query(0) }).is_none());
        assert!(control.controls(&ControlsQuery { role: Some(role), ..ctl_query() }).is_none());
    }
    assert!(control.suspension(&SuspensionQuery { role: None, ..susp_query(0) }).is_none());
    assert!(control.suspension(&susp_query(2)).is_none(), "an axle past 1");
    assert!(control.controls(&ControlsQuery { role: None, ..ctl_query() }).is_none());
}

#[test]
fn a_hook_that_is_not_registered_answers_not_handled() {
    let mut control = running();
    assert!(control.controls(&ctl_query()).is_none(), "the aids and the steering are off");
    control.set_unit(Unit::Suspension, UnitMode::Off);
    accept(&mut control);
    assert!(control.suspension(&susp_query(0)).is_none());
}

#[test]
fn invalid_values_decline_the_suspension_and_controls_hooks() {
    let mut control = running_with(modes(UnitMode::On, UnitMode::On, UnitMode::On, UnitMode::On));
    let mut bad = susp_query(0);
    bad.wheels[0].compression_m = f32::NAN;
    assert!(control.suspension(&bad).is_none());
    assert!(control.suspension(&SuspensionQuery { dt: 0.0, ..susp_query(0) }).is_none());
    assert!(control.suspension(&SuspensionQuery { rest_travel_m: f32::INFINITY, ..susp_query(1) }).is_none());
    assert!(control.suspension(&susp_query(0)).is_some(), "the axle recovers after a reseed");
    assert!(control.controls(&ControlsQuery { steer: f32::NAN, ..ctl_query() }).is_none());
    assert!(control.controls(&ControlsQuery { dt: 1.0, ..ctl_query() }).is_none());
}

#[test]
fn the_suspension_answer_carries_camber_to_the_tyre_and_load_stays_finite() {
    let mut control = running();
    let rest = control.suspension(&susp_query(0)).expect("handled");
    assert!((rest.wheels[0].camber_rad + 0.035).abs() < 1e-6 && (rest.wheels[1].camber_rad + 0.035).abs() < 1e-6);
    assert!((rest.wheels[0].tyre_load_n - 3740.0).abs() < 0.1, "{}", rest.wheels[0].tyre_load_n);
    // The tyre hook converts the camber's sign by side: leaning into the force gives more lateral force than leaning away.
    let flat = control.tyre(&TyreQuery { camber_rad: 0.0, ..query(1) }).expect("handled").lateral_force_n.abs();
    let mut other = running();
    let leaning = other.tyre(&TyreQuery { camber_rad: -0.05, ..query(1) }).expect("handled").lateral_force_n.abs();
    let mut away = running();
    let leaning_away = away.tyre(&TyreQuery { camber_rad: 0.05, ..query(1) }).expect("handled").lateral_force_n.abs();
    assert!(leaning != leaning_away, "the camber moves the force: {leaning} {leaning_away} {flat}");
}

#[test]
fn the_thermal_factor_scales_the_peak_and_a_cold_tyre_grips_less_than_a_warm_one() {
    let mut cold = running_with(modes(UnitMode::Preset, UnitMode::Preset, UnitMode::Preset, UnitMode::On));
    let mut off = running_with(modes(UnitMode::Preset, UnitMode::Preset, UnitMode::Preset, UnitMode::Off));
    let cold_force = cold.tyre(&query(0)).expect("handled").lateral_force_n.abs();
    let plain = off.tyre(&query(0)).expect("handled").lateral_force_n.abs();
    assert!(cold_force < plain, "a tyre at 20 C grips less: {cold_force} against {plain}");
    let mut warm = running_with(Settings {
        warm_start: true,
        ..modes(UnitMode::Preset, UnitMode::Preset, UnitMode::Preset, UnitMode::On)
    });
    warm.reseed();
    let warm_force = warm.tyre(&query(0)).expect("handled").lateral_force_n.abs();
    assert!(warm_force > cold_force, "{warm_force} against {cold_force}");
    // Driving heats the tyre: the grip rises over a few thousand ticks of cornering.
    let mut driven = running();
    let first = driven.tyre(&query(0)).expect("handled").lateral_force_n.abs();
    let mut last = first;
    for _ in 0..6000 {
        for wheel in 0..4 {
            last = driven.tyre(&query(wheel)).map_or(last, |answer| answer.lateral_force_n.abs());
        }
    }
    assert!(last > first, "the tyres warmed up: {first} then {last}");
}

#[test]
fn switching_the_thermal_unit_starts_the_tyres_again() {
    let mut control = running();
    for _ in 0..3000 {
        control.tyre(&query(0));
    }
    let warm = control.tyre(&query(0)).expect("handled").lateral_force_n.abs();
    control.set_unit(Unit::Thermal, UnitMode::Off);
    let plain = control.tyre(&query(0)).expect("handled").lateral_force_n.abs();
    control.set_unit(Unit::Thermal, UnitMode::On);
    let cold = control.tyre(&query(0)).expect("handled").lateral_force_n.abs();
    assert!(cold < warm && cold < plain, "cold {cold}, warm {warm}, plain {plain}");
}

#[test]
fn the_aids_unit_answers_the_pedals_and_the_steering_unit_sets_the_flag() {
    let mut control = running_with(modes(UnitMode::Preset, UnitMode::On, UnitMode::Off, UnitMode::Preset));
    let out = control.controls(&ctl_query()).expect("handled");
    assert_eq!(out.steer_target_rad, None);
    assert!((out.throttle - 1.0).abs() < 1e-6, "no slip, no cut");
    control.run(Command::Level(LevelKind::Tc, 5));
    let mut spinning = ctl_query();
    spinning.slip_ratio = [0.0, 0.0, 0.5, 0.5];
    let mut cut = 1.0;
    for _ in 0..6 {
        cut = control.controls(&spinning).expect("handled").throttle;
    }
    assert!(cut < 0.1, "traction control level 5 cuts the throttle: {cut}");
    control.run(Command::Unit(Unit::Aids, UnitMode::Off));
    control.run(Command::Unit(Unit::Steering, UnitMode::On));
    accept(&mut control);
    let steer = control.controls(&ControlsQuery { steer: 1.0, forward_speed: 0.0, ..ctl_query() }).expect("handled");
    assert!(steer.steer_target_rad.is_some(), "the steering unit sets the flag");
    assert_eq!(steer.brake, [0.5; 4], "with the aids off the pedals pass through");
    assert!(control.ffb_signal().is_finite() && control.column_torque().is_finite());
}

#[test]
fn the_levels_follow_the_preset_until_the_player_moves_a_slider() {
    let mut control = running_with(modes(UnitMode::Preset, UnitMode::On, UnitMode::Off, UnitMode::Preset));
    // The sport preset starts traction control at level 1 and ABS at 1: a spin at slip 0.3 is cut at the first check.
    let mut spinning = ctl_query();
    spinning.slip_ratio = [0.0, 0.0, 0.3, 0.3];
    let mut throttle = 1.0;
    for _ in 0..3 {
        throttle = control.controls(&spinning).expect("handled").throttle;
    }
    assert!(throttle < 1.0, "the preset's level 1 acts: {throttle}");
    control.run(Command::Level(LevelKind::Tc, 0));
    for _ in 0..6 {
        throttle = control.controls(&spinning).expect("handled").throttle;
    }
    assert_eq!(throttle, 1.0, "the player's 0 means off, not the preset's level");
}

#[test]
fn the_new_console_words_parse_and_run() {
    assert_eq!(commands::parse(&["suspension", "off"]), Ok(Command::Unit(Unit::Suspension, UnitMode::Off)));
    assert_eq!(commands::parse(&["aids", "preset"]), Ok(Command::Unit(Unit::Aids, UnitMode::Preset)));
    assert_eq!(commands::parse(&["steering", "on"]), Ok(Command::Unit(Unit::Steering, UnitMode::On)));
    assert_eq!(commands::parse(&["thermal", "on"]), Ok(Command::Unit(Unit::Thermal, UnitMode::On)));
    assert_eq!(commands::parse(&["tc", "3"]), Ok(Command::Level(LevelKind::Tc, 3)));
    assert_eq!(commands::parse(&["bias", "-7"]), Ok(Command::Level(LevelKind::Bias, -7)));
    assert_eq!(commands::parse(&["device", "wheel"]), Ok(Command::Device(crate::steering::Device::Wheel)));
    assert_eq!(commands::parse(&["hud", "on"]), Ok(Command::Hud(true)));
    assert_eq!(commands::parse(&["ffb"]), Ok(Command::Ffb));
    for bad in [
        &["esc", "5"][..],
        &["bias", "25"],
        &["abs", "-1"],
        &["suspension", "maybe"],
        &["device", "joystick"],
        &["hud"],
    ] {
        assert!(commands::parse(bad).is_err(), "{bad:?}");
    }
    let mut control = running();
    assert!(control.run(Command::Unit(Unit::Thermal, UnitMode::Off)).contains("thermal off"));
    let status = control.status();
    assert!(
        status.contains("thermal off")
            && status.contains("suspension on")
            && status.contains("registered (tyre+suspension)"),
        "{status}"
    );
    assert!(control.run(Command::Ffb).contains("force feedback signal"));
    control.run(Command::Level(LevelKind::Bias, -50));
    assert_eq!(control.levels.bias_clicks, Some(-20), "the bias is kept inside its range");
}

#[test]
fn the_hud_line_is_calm_optional_and_needs_the_thermal_unit() {
    let mut control = running();
    assert!(control.hud_line().is_none(), "the HUD setting is off by default");
    control.run(Command::Hud(true));
    assert!(control.hud_line().is_none(), "nothing before the first few seconds of driving");
    let mut lines = Vec::new();
    for _ in 0..(60 * 16) {
        for wheel in 0..4 {
            control.tyre(&query(wheel));
        }
        if let Some(line) = control.hud_line() {
            lines.push(line);
        }
    }
    assert_eq!(lines.len(), 3, "one line every five seconds: {lines:?}");
    assert!(lines[0].starts_with("Tyres (core C):") && lines[0].contains("kPa") && lines[0].contains("life"));
    control.run(Command::Unit(Unit::Thermal, UnitMode::Off));
    for _ in 0..(60 * 6) {
        control.tyre(&query(0));
        assert!(control.hud_line().is_none(), "no thermal unit, no line");
    }
}

#[test]
fn a_car_change_rebuilds_every_unit_from_the_new_spec() {
    let mut control = running_with(modes(UnitMode::On, UnitMode::On, UnitMode::On, UnitMode::On));
    control.suspension(&susp_query(0));
    control.load_spec(CarSpec { revision: 2, mass_kg: 1900.0, ..CarSpec::ref_sport() });
    assert!(control.status().contains("spec-1900-245-300"));
    let out = control.suspension(&SuspensionQuery { vehicle_revision: 2, ..susp_query(0) });
    assert!(out.is_some(), "the hubs reseed from the new car");
}
