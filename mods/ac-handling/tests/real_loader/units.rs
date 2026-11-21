//! The optional units against the real loader: master off registers nothing, each unit toggles its own hook, every hook answers
//! for a real vehicle without a fault, a refused hook does not take the others down, and the settings and menus of the units.

use std::cell::Cell;
use std::rc::Rc;

use blackbox_vehicle::{InputState, Vehicle};

use crate::build::{logged, serial};
use crate::game::{ALL, Rig};
use crate::{car_with, drive, throttle};

const ALL_UNITS: &str = "enabled = true\nsuspension = \"on\"\naids = \"on\"\nsteering = \"on\"\nthermal = \"on\"\ntc = 2\nabs = 2\nesc = 1\n";

fn counts(rig: &Rig) -> (usize, usize, usize) {
    (rig.game.hooks.borrow().len(), rig.game.suspension_hooks.borrow().len(), rig.game.controls_hooks.borrow().len())
}

/// A real car with every hook the mod has registered installed on it.
fn full_car(rig: &Rig, revision: &Rc<Cell<u32>>) -> Vehicle {
    let tyre = rig.game.hooks.borrow().first().map(|(_, hook)| hook.clone());
    let mut car = car_with(tyre.as_ref(), 0, revision);
    if let Some((_, hook)) = rig.game.suspension_hooks.borrow().first() {
        car.set_suspension_model(hook.model(revision.clone()), 0);
    }
    if let Some((_, hook)) = rig.game.controls_hooks.borrow().first() {
        car.set_controls_model(hook.model(revision.clone()), 0);
    }
    car
}

#[test]
fn master_off_registers_nothing_whatever_the_units_say() {
    let _serial = serial();
    let mut rig = Rig::new("units-off", Some(&ALL_UNITS.replace("enabled = true", "enabled = false")), ALL);
    for _ in 0..5 {
        rig.frame();
    }
    assert_eq!(counts(&rig), (0, 0, 0), "{}", rig.game.command(&["status"]));
    assert_eq!(
        (rig.game.register_calls.get(), rig.game.suspension_calls.get(), rig.game.controls_calls.get()),
        (0, 0, 0)
    );
    rig.game.command(&["aids", "on"]);
    rig.frame();
    assert_eq!(counts(&rig), (0, 0, 0), "a unit switch alone registers nothing");
}

#[test]
fn each_unit_toggles_its_own_hook_independently() {
    let _serial = serial();
    let mut rig = Rig::new("units-toggle", None, ALL);
    rig.game.command(&["on"]);
    rig.frame();
    assert_eq!(counts(&rig), (1, 1, 0), "sport preset: tyre and suspension, no aids or steering");
    rig.game.command(&["suspension", "off"]);
    rig.frame();
    assert_eq!(counts(&rig), (1, 0, 0));
    rig.game.command(&["aids", "on"]);
    rig.frame();
    assert_eq!(counts(&rig), (1, 0, 1));
    rig.game.command(&["steering", "on"]);
    rig.game.command(&["aids", "off"]);
    rig.frame();
    assert_eq!(counts(&rig), (1, 0, 1), "the steering unit keeps the controls hook");
    rig.game.command(&["steering", "off"]);
    rig.frame();
    assert_eq!(counts(&rig), (1, 0, 0));
    rig.game.command(&["suspension", "preset"]);
    rig.frame();
    assert_eq!(counts(&rig), (1, 1, 0), "the preset's default is on");
    rig.game.command(&["thermal", "off"]);
    rig.frame();
    assert_eq!(counts(&rig), (1, 1, 0), "thermal has no hook");
    assert!(rig.game.command(&["status"]).contains("thermal off"));
    rig.game.command(&["off"]);
    rig.frame();
    assert_eq!(counts(&rig), (0, 0, 0), "master off takes every hook away");
    assert!(logged("the suspension hook is off") && logged("the tyre hook is off"));
}

#[test]
fn the_settings_lists_and_the_menu_sliders_drive_the_units() {
    let _serial = serial();
    let mut rig = Rig::new("units-settings", None, ALL);
    rig.game.choose("ac_handling_enabled", "on");
    rig.frame();
    assert_eq!(counts(&rig), (1, 1, 0));
    rig.game.choose("ac_handling_suspension", "off");
    rig.game.choose("ac_handling_aids", "on");
    rig.frame();
    assert_eq!(counts(&rig), (1, 0, 1));
    rig.game.choose("ac_handling_aids", "preset");
    rig.frame();
    assert_eq!(counts(&rig), (1, 0, 0));
    let settings: Vec<String> = rig.game.settings.borrow().iter().map(|(id, _)| id.clone()).collect();
    for id in [
        "ac_handling_suspension",
        "ac_handling_aids",
        "ac_handling_steering",
        "ac_handling_thermal",
        "ac_handling_hud",
        "ac_handling_device",
    ] {
        assert!(settings.contains(&id.to_owned()), "{id} in {settings:?}");
    }
    let menus: Vec<String> = rig.game.menus.borrow().iter().map(|(name, _, _)| name.clone()).collect();
    for name in [
        "ac_handling_grip",
        "ac_handling_tc",
        "ac_handling_abs",
        "ac_handling_esc",
        "ac_handling_bias",
        "ac_handling_steer_sens",
        "ac_handling_ffb_gain",
        "ac_handling_ffb_curve",
    ] {
        assert!(menus.contains(&name.to_owned()), "{name} in {menus:?}");
    }
    rig.game.choose("ac_handling_device", "wheel");
    assert!(rig.game.command(&["status"]).contains("on"));
}

#[test]
fn a_real_car_drives_on_all_three_hooks_without_a_fault() {
    let _serial = serial();
    let mut rig = Rig::new("units-drive", Some(ALL_UNITS), ALL);
    rig.frame();
    assert_eq!(counts(&rig), (1, 1, 1), "{}", rig.game.command(&["status"]));
    assert!(logged("the controls hook is on") && logged("the suspension hook is on"));
    let revision = Rc::new(Cell::new(1));
    let mut car = full_car(&rig, &revision);
    assert!(car.has_tyre_model() && car.has_suspension_model() && car.has_controls_model());
    drive(&mut car, 5.0, throttle());
    assert!(car.forward_speed() > 8.0 && car.forward_speed().is_finite(), "{}", car.forward_speed());
    let turning = InputState { throttle: 0.5, steer: 0.4, ..InputState::default() };
    drive(&mut car, 3.0, turning);
    let braking = InputState { brake: 1.0, ..InputState::default() };
    drive(&mut car, 6.0, braking);
    assert!(car.position().is_finite() && car.forward_speed().is_finite());
    assert!(car.forward_speed() < 2.0, "the car stopped under ABS: {}", car.forward_speed());
    assert_eq!(
        (car.tyre_model_faults(), car.suspension_model_faults(), car.controls_model_faults()),
        (0, 0, 0),
        "no hook faulted"
    );
    assert!(car.has_tyre_model() && car.has_suspension_model() && car.has_controls_model(), "none was dropped");
}

#[test]
fn the_steering_flag_changes_the_cars_steering_only_while_the_unit_is_on() {
    let _serial = serial();
    let heading_after = |config: &str| {
        let mut rig = Rig::new("units-steer", Some(config), ALL);
        rig.frame();
        let revision = Rc::new(Cell::new(1));
        let mut car = full_car(&rig, &revision);
        drive(&mut car, 3.0, throttle());
        drive(&mut car, 2.0, InputState { throttle: 0.3, steer: 1.0, ..InputState::default() });
        (car.position().x, car.position().z, car.controls_model_faults())
    };
    let steering = heading_after("enabled = true\nsteering = \"on\"\nsuspension = \"off\"\n");
    let aids_only = heading_after("enabled = true\naids = \"on\"\nsuspension = \"off\"\n");
    assert_eq!((steering.2, aids_only.2), (0, 0));
    assert!(
        steering.0 != aids_only.0 || steering.1 != aids_only.1,
        "the steering model moves the car differently: {steering:?} {aids_only:?}"
    );
}

#[test]
fn a_refused_suspension_hook_does_not_take_the_others_down() {
    let _serial = serial();
    let mut rig = Rig::new("units-refused", Some("enabled = true\naids = \"on\"\n"), ALL);
    rig.game.refuse_suspension.set(true);
    for _ in 0..4 {
        rig.frame();
    }
    assert_eq!(counts(&rig), (1, 0, 1), "the tyre and controls hooks stay");
    assert_eq!(rig.game.suspension_calls.get(), 1, "refused once, not retried every frame");
    assert!(logged("the suspension hook was refused"));
    rig.game.refuse_suspension.set(false);
    rig.game.command(&["suspension", "on"]);
    rig.frame();
    assert_eq!(counts(&rig), (1, 1, 1), "a changed switch tries again");
}

#[test]
fn an_online_session_refuses_every_hook_and_the_mod_stays_off() {
    let _serial = serial();
    let mut rig = Rig::new("units-online", Some(ALL_UNITS), ALL);
    rig.game.online.set(true);
    for _ in 0..3 {
        rig.frame();
    }
    assert_eq!(counts(&rig), (0, 0, 0));
    assert!(rig.game.command(&["status"]).contains("off"));
}

#[test]
fn only_the_player_car_is_handled_by_every_hook() {
    let _serial = serial();
    let mut rig = Rig::new("units-roles", Some(ALL_UNITS), ALL);
    rig.frame();
    let revision = Rc::new(Cell::new(1));
    let mut plain = car_with(None, 0, &revision);
    let mut cop = car_with(None, 2, &revision);
    if let Some((_, hook)) = rig.game.hooks.borrow().first() {
        cop.set_tyre_model(hook.model(revision.clone()), 2);
    }
    if let Some((_, hook)) = rig.game.suspension_hooks.borrow().first() {
        cop.set_suspension_model(hook.model(revision.clone()), 2);
    }
    if let Some((_, hook)) = rig.game.controls_hooks.borrow().first() {
        cop.set_controls_model(hook.model(revision.clone()), 2);
    }
    drive(&mut plain, 3.0, throttle());
    drive(&mut cop, 3.0, throttle());
    let key = |car: &Vehicle| (car.position().x.to_bits(), car.position().z.to_bits(), car.forward_speed().to_bits());
    assert_eq!(key(&plain), key(&cop), "a declined car runs the built-in model, bit for bit");
}
