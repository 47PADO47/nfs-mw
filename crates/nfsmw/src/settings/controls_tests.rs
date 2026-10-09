use super::{Controls, Deadzone, Partial, Sensitivity, Settings, env, file, write};

#[test]
fn response_ranges_exclude_singular_nonfinite_and_zero_sensitivity_values() {
    for input in ["96", "100", "-1", "NaN", "inf", "0.5", "bad"] {
        assert!(input.parse::<Deadzone>().is_err(), "{input}");
    }
    for input in ["0", "401", "-1", "NaN", "inf", "1.5", "bad"] {
        assert!(input.parse::<Sensitivity>().is_err(), "{input}");
    }
    assert_eq!("95%".parse::<Deadzone>().unwrap().percent(), 95);
    assert_eq!("400%".parse::<Sensitivity>().unwrap().percent(), 400);
    for d in [0, 15, 95] {
        let d = Deadzone::new(d).unwrap();
        assert_eq!(d.apply(0.0), 0.0);
        assert_eq!(d.apply(1.0), 1.0);
        assert_eq!(d.apply(-1.0), -1.0);
    }
}

#[test]
fn each_control_resolves_independently_and_bad_values_fall_through() {
    let config = file::parse(
        "steering_deadzone=10\ncamera_deadzone=20\ntrigger_deadzone=5\nsteering_sensitivity=125\ncamera_sensitivity=150\nmouse_sensitivity=50\ninvert_camera_y=true",
        "test",
    );
    let environment = env::read(|name| match name {
        "NFSMW_STEERING_DEADZONE" => Some("25".into()),
        "NFSMW_CAMERA_DEADZONE" => Some("100".into()),
        "NFSMW_MOUSE_SENSITIVITY" => Some("bad".into()),
        _ => None,
    });
    let cli = Partial { trigger_deadzone: Some(Deadzone::new(0).unwrap()), ..Partial::default() };
    let s: Settings = cli.or(environment).or(config).into();
    assert_eq!(s.controls.steering_deadzone.percent(), 25);
    assert_eq!(s.controls.camera_deadzone.percent(), 20);
    assert_eq!(s.controls.trigger_deadzone.percent(), 0);
    assert_eq!(s.controls.steering_sensitivity.percent(), 125);
    assert_eq!(s.controls.camera_sensitivity.percent(), 150);
    assert_eq!(s.controls.mouse_sensitivity.percent(), 50);
    assert!(s.controls.invert_camera_y);
    let bad = file::parse("steering_deadzone=100\ncamera_sensitivity=0\ninvert_camera_y='maybe'", "test");
    let defaults: Settings = bad.into();
    assert_eq!(defaults.controls, Controls::default());
}

#[test]
fn gameplay_rows_persist_controls_without_erasing_bindings() {
    use crate::frontend::input_options::InputSetting;
    let mut settings: Settings = Partial::default().into();
    let mut changes = Partial::default();
    for row in InputSetting::ALL {
        assert!(row.step(&mut settings, &mut changes, true));
    }
    let existing = "game_dir='unchanged'\nunknown=7\n[bindings]\nnos=['button:East']";
    let saved = write::merge(existing, &changes).unwrap();
    let loaded: Settings = file::parse(&saved, "test").into();
    assert_eq!(loaded.controls, settings.controls);
    let table: toml::Table = saved.parse().unwrap();
    assert_eq!(table["game_dir"].as_str(), Some("unchanged"));
    assert_eq!(table["unknown"].as_integer(), Some(7));
    assert_eq!(table["bindings"]["nos"][0].as_str(), Some("button:East"));
}
