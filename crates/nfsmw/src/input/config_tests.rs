use super::{Action, ActionState, Bindings, snapshot::Snapshot, source};
use crate::settings::{Deadzone, DeadzoneMode, Partial, Sensitivity, Settings};
use bevy_input::{
    gamepad::{GamepadAxis, GamepadButton},
    keyboard::KeyCode,
};

fn settings() -> Settings {
    Partial::default().into()
}
fn value(bindings: &Bindings, snapshot: &Snapshot, action: Action) -> f32 {
    let mut state = ActionState::default();
    state.update(bindings, snapshot, false);
    state.value(action)
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-4, "{a} versus {b}");
}

#[test]
fn all_defaults_round_trip_with_signed_axes_clock_and_wheel_codes() {
    let bindings = Bindings::with_paddles(Some(7), Some(13));
    for binding in &bindings.0 {
        let text = source::encode(binding);
        assert_eq!(source::parse(binding.action, &text).unwrap(), *binding, "{text}");
    }
    let text = bindings.merge_config("game_dir='keep me'\nunknown=42").unwrap();
    let mut loaded = Bindings::default();
    loaded.apply_config(&text, "test");
    for action in Action::ALL {
        assert_eq!(
            bindings.0.iter().filter(|b| b.action == action).collect::<Vec<_>>(),
            loaded.0.iter().filter(|b| b.action == action).collect::<Vec<_>>(),
            "{}",
            action.name()
        );
    }
    let table: toml::Table = text.parse().unwrap();
    assert_eq!(table["game_dir"].as_str(), Some("keep me"));
    assert_eq!(table["unknown"].as_integer(), Some(42));
}

#[test]
fn replacing_keyboard_preserves_gamepad_and_changes_live_listing() {
    let mut bindings = Bindings::default();
    bindings.bind(Action::Throttle, "key:J", false).unwrap();
    let mut s = Snapshot::default();
    s.keys.insert(KeyCode::KeyW);
    assert_eq!(value(&bindings, &s, Action::Throttle), 0.0);
    s.keys.insert(KeyCode::KeyJ);
    assert_eq!(value(&bindings, &s, Action::Throttle), 1.0);
    s.keys.clear();
    s.pad_triggers.insert(GamepadButton::RightTrigger2, 0.6);
    close(value(&bindings, &s, Action::Throttle), 0.6);
    assert!(bindings.describe().contains("throttle"));
    assert!(bindings.describe().contains("J"));
}

#[test]
fn bad_override_keeps_defaults_and_empty_array_unbinds() {
    let mut bindings = Bindings::default();
    let before = bindings.0.clone();
    bindings.apply_config("[bindings]\nthrottle=['key:J','invalid:garbage']\nunknown_action=[]", "test");
    assert_eq!(bindings.0, before);
    bindings.apply_config("[bindings]\nthrottle=[]", "test");
    assert!(!bindings.0.iter().any(|b| b.action == Action::Throttle));
    assert!(bindings.0.iter().any(|b| b.action == Action::Brake));
    bindings.reset(Some(Action::Throttle), &settings());
    assert!(bindings.0.iter().any(|b| b.action == Action::Throttle));
}

#[test]
fn invalid_live_edits_are_atomic_and_invalid_existing_toml_is_preserved() {
    let mut bindings = Bindings::default();
    for input in [
        "key:NoSuchKey",
        "axis:LeftStickX:NaN",
        "axis:LeftStickX:inf",
        "key:J:0",
        "key:J:10001",
        "mouse:look_x:1:per_second",
        "axis:LeftStickX:1:bad",
    ] {
        let before = bindings.0.clone();
        assert!(bindings.bind(Action::Steer, input, false).is_err(), "{input}");
        assert_eq!(bindings.0, before);
    }
    assert!(Action::parse("bad").is_err());
    assert!(bindings.merge_config("broken=[").is_err());
    assert!(bindings.merge_config("bindings=3").is_err());
}

#[test]
fn replacing_a_new_device_family_cannot_exceed_the_saved_binding_limit() {
    let mut bindings = Bindings::default();
    bindings.unbind(Action::Throttle, None);
    for code in 0..32 {
        bindings.bind(Action::Throttle, &format!("button:Other({code})"), true).unwrap();
    }
    let before = bindings.0.clone();
    assert!(bindings.bind(Action::Throttle, "key:J", false).is_err());
    assert_eq!(bindings.0, before);
    bindings.bind(Action::Throttle, "button:South", false).unwrap();
    assert_eq!(bindings.0.iter().filter(|b| b.action == Action::Throttle).count(), 1);
}

#[test]
fn defaults_keep_existing_linear_rescaled_curve() {
    let bindings = Bindings::default();
    for raw in [-1.0_f32, -0.5, -0.15, 0.0, 0.1, 0.15, 0.5, 1.0] {
        let mut s = Snapshot::default();
        s.pad_axes.insert(GamepadAxis::LeftStickX, raw);
        let expected = match raw.abs() <= 0.15 {
            true => 0.0,
            false => raw.signum() * (raw.abs() - 0.15) / 0.85,
        };
        close(value(&bindings, &s, Action::Steer), expected);
    }
}

#[test]
fn cutoff_preserves_values_outside_each_independent_deadzone() {
    let bindings = Bindings::default();
    let mut s = Snapshot::default();
    s.controls.deadzone_mode = DeadzoneMode::Cutoff;
    s.controls.steering_deadzone = Deadzone::new(10).unwrap();
    s.controls.trigger_deadzone = Deadzone::new(10).unwrap();
    for raw in [-1.0_f32, -0.5, -0.1, -0.09, 0.0, 0.09, 0.1, 0.5, 1.0] {
        s.pad_axes.insert(GamepadAxis::LeftStickX, raw);
        let expected = match raw.abs() < 0.1 {
            true => 0.0,
            false => raw,
        };
        close(value(&bindings, &s, Action::Steer), expected);
        s.pad_triggers.insert(GamepadButton::RightTrigger2, raw.abs());
        close(value(&bindings, &s, Action::Throttle), expected.abs());
    }
}

#[test]
fn saved_bindings_reload_and_settings_writes_preserve_them() {
    let name = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("nfsmw-bindings-{}-{name}.toml", std::process::id()));
    let mut bindings = Bindings::default();
    bindings.bind(Action::Throttle, "key:J", false).unwrap();
    bindings.bind(Action::Nos, "button:b", false).unwrap();
    std::fs::write(&path, "game_dir='preserve'\n[bindings]\nfuture_action=['future:value']").unwrap();
    bindings.save_to(&path).unwrap();
    crate::settings::write_file(
        &path,
        &Partial {
            deadzone_mode: Some(DeadzoneMode::Cutoff),
            steering_deadzone: Some(Deadzone::new(4).unwrap()),
            ..Partial::default()
        },
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let mut loaded = Bindings::default();
    loaded.apply_config(&text, "temporary file");
    assert_eq!(loaded.merge_config("").unwrap(), bindings.merge_config("").unwrap());
    let table: toml::Table = text.parse().unwrap();
    assert_eq!(table["game_dir"].as_str(), Some("preserve"));
    assert_eq!(table["bindings"]["future_action"][0].as_str(), Some("future:value"));
    assert_eq!(table["steering_deadzone"].as_integer(), Some(4));
    assert_eq!(table["deadzone_mode"].as_str(), Some("cutoff"));
    std::fs::write(&path, "broken=[").unwrap();
    assert!(bindings.save_to(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken=[");
    std::fs::remove_file(path).unwrap();
}

#[test]
fn independent_responses_keep_analog_endpoints_and_full_digital_steering() {
    let bindings = Bindings::default();
    let mut s = Snapshot::default();
    s.controls.steering_deadzone = Deadzone::new(10).unwrap();
    s.controls.camera_deadzone = Deadzone::new(30).unwrap();
    s.controls.steering_sensitivity = Sensitivity::new(150).unwrap();
    s.pad_axes.insert(GamepadAxis::LeftStickX, 0.5);
    s.pad_axes.insert(GamepadAxis::RightStickX, 0.2);
    s.dt = 1.0;
    close(value(&bindings, &s, Action::Steer), 2.0 / 3.0);
    assert_eq!(value(&bindings, &s, Action::LookX), 0.0);
    s.pad_axes.insert(GamepadAxis::LeftStickX, 1.0);
    assert_eq!(value(&bindings, &s, Action::Steer), 1.0);
    s.pad_axes.clear();
    s.controls.steering_sensitivity = Sensitivity::new(25).unwrap();
    s.keys.insert(KeyCode::KeyD);
    assert_eq!(value(&bindings, &s, Action::Steer), 1.0);
}

#[test]
fn trigger_deadzone_keeps_full_pull_and_rejects_nonfinite_values() {
    let bindings = Bindings::default();
    let mut s = Snapshot::default();
    s.controls.trigger_deadzone = Deadzone::new(10).unwrap();
    for (raw, expected) in [(0.05, 0.0), (0.1, 0.0), (0.55, 0.5), (1.0, 1.0)] {
        s.pad_triggers.insert(GamepadButton::RightTrigger2, raw);
        close(value(&bindings, &s, Action::Throttle), expected);
    }
    s.pad_triggers.insert(GamepadButton::RightTrigger2, f32::NAN);
    assert_eq!(value(&bindings, &s, Action::Throttle), 0.0);
}

#[test]
fn camera_inverts_once_mouse_remains_per_frame_stick_per_second() {
    let bindings = Bindings::default();
    let mut s = Snapshot { dt: 0.0, mouse_captured: true, mouse_delta: (8.0, 4.0), ..Snapshot::default() };
    s.controls.mouse_sensitivity = Sensitivity::new(50).unwrap();
    s.controls.invert_camera_y = true;
    assert_eq!(value(&bindings, &s, Action::LookX), 4.0);
    assert_eq!(value(&bindings, &s, Action::LookY), -2.0);
    s.mouse_delta = (0.0, 0.0);
    s.dt = 1.0;
    s.pad_axes.insert(GamepadAxis::RightStickY, 1.0);
    s.controls.camera_sensitivity = Sensitivity::new(200).unwrap();
    assert_eq!(value(&bindings, &s, Action::LookY), 1400.0);
    s.controls.invert_camera_y = false;
    s.pad_axes.clear();
    s.pad_axes.insert(GamepadAxis::RightStickX, 1.0);
    for rate in [30, 60, 120, 240] {
        s.dt = 1.0 / rate as f32;
        close(value(&bindings, &s, Action::LookX) * rate as f32, 1400.0);
    }
}

#[test]
fn raw_axis_bounds_and_xtended_style_rebindings_are_safe_under_ui_focus() {
    let mut bindings = Bindings::default();
    let mut s = Snapshot::default();
    for raw in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        s.pad_axes.insert(GamepadAxis::LeftStickX, raw);
        assert_eq!(value(&bindings, &s, Action::Steer), 0.0);
    }
    s.pad_axes.insert(GamepadAxis::LeftStickX, 4.0);
    assert_eq!(value(&bindings, &s, Action::Steer), 1.0);
    s.pad_axes.clear();
    bindings.bind(Action::Steer, "key:D", false).unwrap();
    bindings.bind(Action::Steer, "key:A:-1", true).unwrap();
    bindings.bind(Action::Nos, "button:b", false).unwrap();
    s.keys.insert(KeyCode::KeyA);
    s.pad_buttons.insert(GamepadButton::East);
    assert_eq!(value(&bindings, &s, Action::Steer), -1.0);
    assert_eq!(value(&bindings, &s, Action::Nos), 1.0);
    let mut state = ActionState::default();
    state.update(&bindings, &s, true);
    assert_eq!(state.value(Action::Nos), 0.0);
}

#[test]
fn wheel_pedal_axes_parse_round_trip_and_drive_the_actions() {
    let throttle = source::parse(Action::Throttle, "pedal:Other(2)").unwrap();
    assert_eq!(source::encode(&throttle), "pedal:Other(2)");
    let brake = source::parse(Action::Brake, "pedal_inv:Other(5)").unwrap();
    assert_eq!(source::encode(&brake), "pedal_inv:Other(5)");
    assert_eq!(source::parse(Action::Brake, &source::encode(&brake)).unwrap(), brake);
    assert!(source::parse(Action::Brake, "pedal:Nope").is_err());

    let mut bindings = Bindings::default();
    bindings.bind(Action::Throttle, "pedal:Other(2)", false).unwrap();
    bindings.bind(Action::Brake, "pedal_inv:Other(5)", false).unwrap();
    bindings.bind(Action::Clutch, "pedal_inv:RightZ", false).unwrap();
    let mut s = Snapshot::default();
    s.pad_axes.insert(GamepadAxis::Other(2), 0.0);
    s.pad_axes.insert(GamepadAxis::Other(5), 1.0);
    s.pad_axes.insert(GamepadAxis::RightZ, -1.0);
    close(value(&bindings, &s, Action::Throttle), 0.5);
    close(value(&bindings, &s, Action::Brake), 0.0);
    close(value(&bindings, &s, Action::Clutch), 1.0);
    s.pad_axes.insert(GamepadAxis::Other(5), -1.0);
    close(value(&bindings, &s, Action::Brake), 1.0);
    assert!(bindings.describe().contains("Pad pedal axis Other(2)"));
}

#[test]
fn gear_and_clutch_bindings_come_from_the_config_file() {
    let defaults = Bindings::default();
    for action in [Action::GearReverse, Action::GearNeutral, Action::Gear1, Action::Gear7] {
        assert!(!defaults.0.iter().any(|b| b.action == action), "{} has no default", action.name());
    }
    let mut bindings = Bindings::default();
    bindings.apply_config(
        "[bindings]\ngear_1 = ['button:Other(20)']\ngear_reverse = ['button:Other(26)']\nclutch = ['pedal_inv:Other(3)']",
        "test",
    );
    let mut s = Snapshot::default();
    s.pad_buttons.insert(GamepadButton::Other(20));
    assert_eq!(value(&bindings, &s, Action::Gear1), 1.0);
    assert_eq!(value(&bindings, &s, Action::Gear2), 0.0);
    assert_eq!(value(&bindings, &s, Action::GearReverse), 0.0);
    s.keys.insert(KeyCode::KeyZ);
    assert_eq!(value(&bindings, &s, Action::Clutch), 0.0, "the config replaced the default Z key");
    assert_eq!(value(&Bindings::default(), &s, Action::Clutch), 1.0, "Z is the default clutch key");
}
