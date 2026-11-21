//! End-to-end test of the mod against the real loader and a fake game: build the `ac-handling` library with cargo, load it,
//! and check what the player's switches do. The fake game offers every interface the mod uses (`bb.handling`, `bb.vehicles`
//! 1.3 with a spec, `bb.config`, `bb.settings`, `bb.menus`, `bb.input`, `bb.commands`, `bb.hud`), keeps the closures the mod
//! registers so the test can press the hotkey and run the command, and drives a real vehicle with the hook the mod registered.
//!
//! The library is built through cargo into its own target folder, like `libs/blackbox-modding/tests/handling_probe.rs` does for
//! its fixture: a `cargo test` of the package builds only the `rlib` of its library target, and the loader needs the `cdylib`.
//! The build uses the LLVM backend whatever the profile says (the repository's dev profile compiles with Cranelift, which cannot
//! catch a panic in a mod DLL on Windows).

mod build;
mod game;
mod units;

use std::cell::Cell;
use std::rc::Rc;

use blackbox_modding::host::TyreHook;
use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle, VehicleSpec};

use build::{ID, logged, serial};
use game::{ALL, Offer, Rig};

/// A car resting on flat ground, with `hook` as its tyre model for a vehicle of `role` and `revision` as its spec revision.
fn car_with(hook: Option<&TyreHook>, role: u32, revision: &Rc<Cell<u32>>) -> Vehicle {
    let ground = FlatGround::new(0.0);
    let mut car = Vehicle::new(VehicleSpec::example());
    assert!(car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
    for _ in 0..180 {
        car.step(FIXED_STEP, &InputState::default(), &ground);
    }
    if let Some(hook) = hook {
        car.set_tyre_model(hook.model(revision.clone()), role);
    }
    car
}

fn drive(car: &mut Vehicle, seconds: f32, input: InputState) {
    let ground = FlatGround::new(0.0);
    for _ in 0..(seconds / FIXED_STEP) as usize {
        car.step(FIXED_STEP, &input, &ground);
    }
}

fn throttle() -> InputState {
    InputState { throttle: 1.0, ..InputState::default() }
}

fn fingerprint(car: &Vehicle) -> Vec<u32> {
    let p = car.position();
    let mut bits = vec![p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), car.forward_speed().to_bits()];
    bits.extend((0..4).map(|i| car.wheel(i).simulated_angular_velocity.to_bits()));
    bits
}

#[test]
fn the_mod_loads_with_the_model_off_and_adds_its_switches() {
    let _serial = serial();
    let mut rig = Rig::new("defaults", None, ALL);
    assert_eq!(rig.runtime.loaded().len(), 1, "rejected: {:?}", rig.runtime.rejected());
    assert!(logged("ac-handling: loaded (ac-handling: off, hook not registered"));
    rig.frame();
    rig.frame();
    assert!(rig.game.hooks.borrow().is_empty(), "off by default: no hook is registered");
    let settings = rig.game.settings.borrow();
    let ids: Vec<&str> = settings.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "ac_handling_enabled",
            "ac_handling_preset",
            "ac_handling_suspension",
            "ac_handling_aids",
            "ac_handling_steering",
            "ac_handling_thermal",
            "ac_handling_hud",
            "ac_handling_device"
        ]
    );
    assert_eq!(rig.game.menus.borrow()[0].0, "ac_handling_grip");
    let hotkeys = rig.game.hotkeys.borrow();
    assert_eq!((hotkeys[0].0.as_str(), hotkeys[0].1.as_str()), ("ac_handling_toggle", "key:F1"));
    assert!(rig.game.command(&["status"]).contains("off"));
}

#[test]
fn the_console_command_switches_the_hook_at_the_next_frame_and_the_car_drives_on_it() {
    let _serial = serial();
    let mut rig = Rig::new("command", None, ALL);
    let revision = Rc::new(Cell::new(1));
    assert!(rig.game.command(&["on"]).contains("on"));
    assert!(rig.game.hooks.borrow().is_empty(), "the command only asks: the hook goes in at the frame boundary");
    rig.frame();
    assert_eq!(rig.game.hooks.borrow().len(), 1);
    assert_eq!(rig.game.hook().owner(), ID);
    assert!(rig.game.hud.borrow().iter().any(|text| text.contains("on")), "{:?}", rig.game.hud.borrow());
    assert!(
        logged("ac-handling: car \"spec-1450-245-300\": L=1.00 -> sport/sport"),
        "the spec was read and the car logged"
    );

    let hook = rig.game.hook();
    let mut car = car_with(Some(&hook), 0, &revision);
    drive(&mut car, 5.0, throttle());
    assert!(car.forward_speed() > 6.0 && car.forward_speed().is_finite(), "{}", car.forward_speed());
    assert!((0..4).all(|i| car.wheel(i).simulated_angular_velocity.is_finite()));
    assert_eq!(car.tyre_model_faults(), 0);
    assert_eq!(hook.stats().faults(), 0);
    assert!(hook.stats().calls() >= 4 * 290, "one call per wheel per tick: {}", hook.stats().calls());
    assert!(rig.game.command(&["status"]).contains("hook registered"));

    rig.game.command(&["off"]);
    assert_eq!(rig.game.hooks.borrow().len(), 1, "still registered until the frame boundary");
    rig.frame();
    assert!(rig.game.hooks.borrow().is_empty());
    assert!(rig.game.hud.borrow().iter().any(|text| text.contains("off")));
    let calls = hook.stats().calls();
    drive(&mut car, 0.5, throttle());
    assert_eq!(hook.stats().calls(), calls, "an unregistered hook is retired: the built-in tyres run");
    assert!(!car.has_tyre_model());
}

#[test]
fn the_master_toggle_the_hotkey_and_the_preset_list_work_live() {
    let _serial = serial();
    let mut rig = Rig::new("switches", None, ALL);
    rig.game.choose("ac_handling_enabled", "on");
    rig.frame();
    assert_eq!(rig.game.hooks.borrow().len(), 1, "the toggle registers the hook");
    rig.game.choose("ac_handling_enabled", "off");
    rig.frame();
    assert!(rig.game.hooks.borrow().is_empty());
    rig.game.press_hotkey();
    rig.frame();
    assert_eq!(rig.game.hooks.borrow().len(), 1, "the hotkey flips it on");
    rig.game.choose("ac_handling_preset", "slick");
    assert!(rig.game.command(&["status"]).contains("preset slick"));
    assert!(rig.game.command(&["status"]).contains("slick/sport"), "the parameter set is rebuilt for the preset");
    assert!(rig.game.command(&["preset", "road"]).contains("road"));
    assert!(rig.game.command(&["preset", "gold"]).contains("usage"));
    rig.game.press_hotkey();
    rig.frame();
    assert!(rig.game.hooks.borrow().is_empty(), "and off again");
    assert_eq!(rig.game.register_calls.get(), 2, "one registration per switch on");
}

#[test]
fn only_the_player_car_is_handled() {
    let _serial = serial();
    let mut rig = Rig::new("roles", None, ALL);
    rig.game.command(&["on"]);
    rig.frame();
    let hook = rig.game.hook();
    let revision = Rc::new(Cell::new(1));
    let mut plain = car_with(None, 0, &revision);
    let mut cop = car_with(Some(&hook), 2, &revision);
    drive(&mut plain, 3.0, throttle());
    drive(&mut cop, 3.0, throttle());
    assert!(hook.stats().calls() > 0, "the hook was asked");
    assert_eq!(fingerprint(&plain), fingerprint(&cop), "a declined car runs the built-in tyres, bit for bit");
    assert_eq!(hook.stats().faults(), 0);
}

#[test]
fn a_new_car_revision_rebuilds_the_parameters_once_and_logs_the_new_car() {
    let _serial = serial();
    let mut rig = Rig::new("revision", None, ALL);
    rig.game.command(&["on"]);
    rig.frame();
    let hook = rig.game.hook();
    let revision = Rc::new(Cell::new(1));
    let mut car = car_with(Some(&hook), 0, &revision);
    drive(&mut car, 0.5, throttle());
    rig.frame();
    assert!(!logged("spec-1900-245-300"));
    // The game swaps the car: a new revision and a heavier spec. The hook sees the number first; the next frame reads the spec.
    rig.game.spec_revision.set(2);
    rig.game.mass.set(1900.0);
    revision.set(2);
    drive(&mut car, 0.5, throttle());
    assert_eq!(car.tyre_model_faults(), 0, "the old parameters answered the tick of the swap");
    rig.frame();
    assert!(logged("ac-handling: car \"spec-1900-245-300\""), "the new spec was read at the frame start");
    drive(&mut car, 1.0, throttle());
    assert_eq!(car.tyre_model_faults(), 0);
}

#[test]
fn a_refused_registration_is_reported_once_and_not_retried() {
    let _serial = serial();
    let mut rig = Rig::new("online", None, ALL);
    rig.game.online.set(true);
    rig.game.command(&["on"]);
    for _ in 0..5 {
        rig.frame();
    }
    assert!(rig.game.hooks.borrow().is_empty());
    assert_eq!(rig.game.register_calls.get(), 1, "refused once, not retried every frame");
    assert!(logged("bb.handling refused in an online session"));
    assert!(logged("the tyre hook was refused"));
    assert!(rig.game.hud.borrow().iter().any(|text| text.contains("refused")));
    assert!(rig.game.command(&["status"]).contains("off"));
}

#[test]
fn the_config_file_can_switch_the_model_on_and_a_bad_config_is_rejected_whole() {
    let _serial = serial();
    let mut rig =
        Rig::new("config-on", Some("enabled = true\npreset = \"road\"\ngrip_gain = 1.1\nsubsteps = 5\n"), ALL);
    rig.frame();
    assert_eq!(rig.game.hooks.borrow().len(), 1);
    assert!(rig.game.command(&["status"]).contains("preset road"));
    drop(rig);

    let mut bad = Rig::new("config-bad", Some("mystery = 1\nenabled = true\n"), ALL);
    assert_eq!(bad.runtime.loaded().len(), 1, "a bad config does not stop the mod");
    assert!(logged("config.toml rejected"));
    bad.frame();
    assert!(bad.game.hooks.borrow().is_empty(), "the defaults stand: off");
}

#[test]
fn the_mod_works_without_the_optional_interfaces() {
    let _serial = serial();
    let mut rig = Rig::new("bare", Some("enabled = true\n"), Offer { switches: false });
    assert_eq!(rig.runtime.loaded().len(), 1, "rejected: {:?}", rig.runtime.rejected());
    assert!(logged("bb.settings is absent"));
    rig.frame();
    assert_eq!(rig.game.hooks.borrow().len(), 1, "no settings, no hud, no console: the config still switches it on");
}

#[test]
fn unloading_the_mod_removes_the_hook_and_a_car_that_still_holds_it_stops_calling() {
    let _serial = serial();
    let mut rig = Rig::new("unload", None, ALL);
    rig.game.command(&["on"]);
    rig.frame();
    let hook = rig.game.hook();
    let revision = Rc::new(Cell::new(1));
    let mut car = car_with(Some(&hook), 0, &revision);
    drive(&mut car, 0.5, throttle());
    let calls = hook.stats().calls();
    assert!(calls > 0);
    rig.runtime.unload_all();
    assert!(rig.game.hooks.borrow().is_empty(), "the mod unregistered its hook");
    assert!(logged("ac-handling: unloaded"));
    drive(&mut car, 0.5, throttle());
    assert_eq!(hook.stats().calls(), calls, "the library is gone: the retired hook is not called");
    assert!(!car.has_tyre_model());
}

#[test]
fn the_car_accelerates_smoothly_on_the_hook_with_the_hosts_drivetrain() {
    // The host's drivetrain feeds torque back from the wheel spin the tyre returns. A tyre model with a lighter wheel than the
    // host's own limit-cycles it (the car crawls at 2 m/s with the rear spin swinging in sign): the shipped presets keep the
    // host's wheel inertia (NOTES.md, spec question 1). This pins the result on a real vehicle.
    let _serial = serial();
    let mut rig = Rig::new("drivetrain", None, ALL);
    rig.game.command(&["on"]);
    rig.frame();
    let hook = rig.game.hook();
    let revision = Rc::new(Cell::new(1));
    let mut car = car_with(Some(&hook), 0, &revision);
    let mut plain = car_with(None, 0, &revision);
    let mut last = 0.0_f32;
    for second in 1..=6 {
        drive(&mut car, 1.0, throttle());
        drive(&mut plain, 1.0, throttle());
        let speed = car.forward_speed();
        assert!(speed > last + 2.0, "second {second}: {speed} after {last}: the car keeps accelerating");
        assert!(
            (0..4).all(|i| car.wheel(i).simulated_angular_velocity > 0.0),
            "no wheel runs backwards at second {second}"
        );
        last = speed;
    }
    assert!(last > 0.6 * plain.forward_speed(), "{last} against the built-in tyres' {}", plain.forward_speed());
    assert_eq!(car.tyre_model_faults(), 0);
}
