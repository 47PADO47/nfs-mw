//! The look of a bike's helmet camera: the pad's right stick X, the mouse's speed and the look keys, dead-zoned and scaled as the
//! other camera looks are. The stick's Y is also the gear shift of the driving profile: the helmet camera takes it away.

use bevy_input::gamepad::GamepadAxis;
use bevy_input::keyboard::KeyCode;

use super::action::Action;
use super::bindings::{Gate, Source};
use super::snapshot::Snapshot;
use super::state::{ActionState, Bindings};
use crate::viewer::camera::head_look::HeadLook;

fn look_of_stick(x: f32, y: f32) -> (f32, f32, bool, bool) {
    let mut state = ActionState::default();
    let mut snap = Snapshot::default();
    snap.pad_axes.insert(GamepadAxis::RightStickX, x);
    snap.pad_axes.insert(GamepadAxis::RightStickY, y);
    state.update(&Bindings::default(), &snap, false);
    (
        state.value(Action::HeadLookX),
        state.value(Action::HeadLookY),
        state.value(Action::ShiftUp) > 0.5,
        state.value(Action::ShiftDown) > 0.5,
    )
}

fn sources(action: Action) -> Vec<Source> {
    Bindings::default().0.iter().filter(|b| b.action == action).map(|b| b.source).collect()
}

/// The two look axes the default bindings read from `snap`.
fn axes(snap: &Snapshot) -> [f32; 2] {
    let mut state = ActionState::default();
    state.update(&Bindings::default(), snap, false);
    [state.value(Action::HeadLookX), state.value(Action::HeadLookY)]
}

/// A captured mouse that moved `pixels` along x and y in a frame of `dt` seconds.
fn mouse(pixels: (f32, f32), dt: f32) -> Snapshot {
    Snapshot { mouse_delta: pixels, mouse_captured: true, dt, ..Snapshot::default() }
}

#[test]
fn the_right_stick_looks_both_ways_and_its_y_is_also_the_gear_shift_the_helmet_camera_takes_away() {
    let pad = |action| {
        sources(action)
            .into_iter()
            .filter(|s| matches!(s, Source::PadAxis(_) | Source::PadButton(_)))
            .collect::<Vec<_>>()
    };
    assert_eq!(pad(Action::HeadLookX), vec![Source::PadAxis(GamepadAxis::RightStickX)]);
    assert_eq!(pad(Action::HeadLookY), vec![Source::PadAxis(GamepadAxis::RightStickY)]);
    assert!(sources(Action::ShiftUp).contains(&Source::PadAxis(GamepadAxis::RightStickY)));
    assert!(sources(Action::ShiftDown).contains(&Source::PadAxis(GamepadAxis::RightStickY)));
    // Pushed up or down the stick looks up or down (and, for any other camera, still shifts); sideways it looks and does not shift.
    assert_eq!(look_of_stick(0.0, 1.0), (0.0, 1.0, true, false));
    assert_eq!(look_of_stick(0.0, -1.0), (0.0, -1.0, false, true));
    assert_eq!(look_of_stick(1.0, 0.0), (1.0, 0.0, false, false));
    assert_eq!(look_of_stick(-1.0, 0.0), (-1.0, 0.0, false, false));
}

#[test]
fn the_helmet_camera_takes_the_sticks_shift_away_and_keeps_the_keys_and_buttons_shifting() {
    let resolve = |keys: &[KeyCode], y: f32| {
        let mut state = ActionState::default();
        let mut snap = Snapshot::default();
        snap.keys.extend(keys.iter().copied());
        snap.pad_axes.insert(GamepadAxis::RightStickY, y);
        state.update(&Bindings::default(), &snap, false);
        state
    };
    let stick_up = resolve(&[], 1.0);
    assert!(stick_up.pressed(Action::ShiftUp) && stick_up.value(Action::HeadLookY) == 1.0);
    let helmet = stick_up.without_stick_shift();
    assert!(!helmet.pressed(Action::ShiftUp) && !helmet.pressed(Action::ShiftDown));
    assert_eq!(helmet.value(Action::HeadLookY), 1.0, "the look is kept");
    let stick_down = resolve(&[], -1.0).without_stick_shift();
    assert!(!stick_down.pressed(Action::ShiftDown) && stick_down.value(Action::HeadLookY) == -1.0);
    // The keys shift the same frame the stick looks, and a stick at rest changes nothing.
    let both = resolve(&[KeyCode::KeyE, KeyCode::KeyQ], 1.0).without_stick_shift();
    assert!(both.pressed(Action::ShiftUp) && both.pressed(Action::ShiftDown), "E and Q still shift");
    let key_only = resolve(&[KeyCode::KeyE], 0.0);
    assert_eq!(key_only.without_stick_shift().value(Action::ShiftUp), key_only.value(Action::ShiftUp));
}

#[test]
fn the_camera_dead_zone_keeps_a_resting_stick_from_turning_the_head_and_the_rim_reaches_the_limit() {
    assert_eq!(look_of_stick(0.0, 0.0).0, 0.0);
    assert_eq!(look_of_stick(0.1, 0.0).0, 0.0, "inside the dead zone (15 percent by default)");
    assert_eq!(look_of_stick(-0.14, 0.0).0, 0.0);
    let (x, ..) = look_of_stick(0.575, 0.0);
    assert!(x > 0.4 && x < 0.6, "rescaled past the dead zone: {x}");
    assert!((look_of_stick(1.0, 0.0).0 - 1.0).abs() < 1e-6);
    assert_eq!(look_of_stick(f32::NAN, 0.0).0, 0.0);
}

#[test]
fn the_actions_round_trip_by_name() {
    assert_eq!(Action::parse("head_look_x"), Ok(Action::HeadLookX));
    assert_eq!(Action::parse("head_look_y"), Ok(Action::HeadLookY));
    assert_eq!(Action::HeadLookX.name(), "head_look_x");
    assert!(Action::ALL.contains(&Action::HeadLookY));
}

#[test]
fn the_mouse_is_a_stick_whose_deflection_is_its_speed_whatever_the_frame_rate() {
    // 700 pixels a second is the end of the stick (the speed a pushed stick turns the free camera by), moving right.
    for fps in [30.0_f32, 60.0, 144.0, 240.0] {
        let [x, y] = axes(&mouse((700.0 / fps, 0.0), 1.0 / fps));
        assert!((x - 1.0).abs() < 1e-4 && y == 0.0, "{fps} fps: {x} {y}");
        let [x, y] = axes(&mouse((-350.0 / fps, 0.0), 1.0 / fps));
        assert!((x + 0.5).abs() < 1e-4 && y == 0.0, "{fps} fps: {x} {y}");
    }
    // Up on the desk (negative y) looks up, down looks down.
    let [x, up] = axes(&mouse((0.0, -7.0), 0.01));
    assert!(x == 0.0 && (up - 1.0).abs() < 1e-4, "{x} {up}");
    let [_, down] = axes(&mouse((0.0, 3.5), 0.01));
    assert!((down + 0.5).abs() < 1e-4, "{down}");
    // A flick faster than the end of the stick is held to it.
    assert_eq!(axes(&mouse((500.0, -500.0), 0.01)), [1.0, 1.0]);
}

#[test]
fn the_mouse_gives_nothing_without_its_gate_motion_or_time_and_the_other_looks_keep_their_pixels() {
    let mut snap = mouse((10.0, 10.0), 0.01);
    snap.mouse_captured = false;
    assert_eq!(axes(&snap), [0.0, 0.0], "an uncaptured cursor (a menu, the pointer) does not look");
    snap.buttons.insert(bevy_input::mouse::MouseButton::Right);
    assert!(axes(&snap)[0] > 0.0, "the right button is the other half of the gate, as for the free look");
    assert_eq!(axes(&mouse((0.0, 0.0), 0.01)), [0.0, 0.0], "a mouse that does not move is a stick at rest");
    for dt in [0.0, -1.0, f32::NAN] {
        assert_eq!(axes(&mouse((10.0, 10.0), dt)), [0.0, 0.0], "dt {dt}");
    }
    // The free camera's own look still reads the pixels of the frame, not a speed.
    let mut state = ActionState::default();
    state.update(&Bindings::default(), &mouse((10.0, -4.0), 0.01), false);
    assert_eq!((state.value(Action::LookX), state.value(Action::LookY)), (10.0, -4.0));
    // And the driving actions do not see the mouse's look.
    for action in [Action::Steer, Action::Throttle, Action::Brake, Action::ShiftUp, Action::ShiftDown] {
        assert_eq!(state.value(action), 0.0, "{}", action.name());
    }
}

#[test]
fn the_keys_hold_the_stick_at_its_end_and_a_key_pair_cancels() {
    let keys = |held: &[KeyCode]| {
        let mut snap = Snapshot::default();
        snap.keys.extend(held.iter().copied());
        axes(&snap)
    };
    assert_eq!(keys(&[]), [0.0, 0.0]);
    assert_eq!(keys(&[KeyCode::Numpad6]), [1.0, 0.0]);
    assert_eq!(keys(&[KeyCode::Numpad4]), [-1.0, 0.0]);
    assert_eq!(keys(&[KeyCode::BracketRight]), [1.0, 0.0]);
    assert_eq!(keys(&[KeyCode::BracketLeft]), [-1.0, 0.0]);
    assert_eq!(keys(&[KeyCode::Numpad9]), [0.0, 1.0]);
    assert_eq!(keys(&[KeyCode::Numpad3]), [0.0, -1.0]);
    assert_eq!(keys(&[KeyCode::Equal]), [0.0, 1.0]);
    assert_eq!(keys(&[KeyCode::Minus]), [0.0, -1.0]);
    assert_eq!(keys(&[KeyCode::Equal, KeyCode::Minus]), [0.0, 0.0]);
    assert_eq!(keys(&[KeyCode::Numpad4, KeyCode::Numpad6]), [0.0, 0.0]);
    // Both a key and the numpad's twin do not push past the end of the stick.
    assert_eq!(keys(&[KeyCode::Numpad6, KeyCode::BracketRight]), [1.0, 0.0]);
}

#[test]
fn no_look_key_is_a_driving_input_and_the_rider_weight_keys_stay_theirs() {
    let look_keys = [
        KeyCode::Numpad4,
        KeyCode::Numpad6,
        KeyCode::Numpad9,
        KeyCode::Numpad3,
        KeyCode::BracketLeft,
        KeyCode::BracketRight,
        KeyCode::Equal,
        KeyCode::Minus,
    ];
    for b in Bindings::default().0.iter().filter(|b| matches!(b.source, Source::Key(k) if look_keys.contains(&k))) {
        assert!(matches!(b.action, Action::HeadLookX | Action::HeadLookY), "{} on a look key", b.action.name());
    }
    for key in [KeyCode::Numpad8, KeyCode::Numpad2] {
        let on =
            Bindings::default().0.iter().filter(|b| b.source == Source::Key(key)).map(|b| b.action).collect::<Vec<_>>();
        assert_eq!(on, vec![Action::RiderWeight], "{key:?}");
    }
}

#[test]
fn the_mouse_defaults_are_the_gated_motion_with_the_scale_of_the_speed_and_round_trip_in_the_config() {
    let mouse_looks: Vec<_> =
        Bindings::default().0.into_iter().filter(|b| matches!(b.source, Source::MouseMotion { .. })).collect();
    let on = |action| mouse_looks.iter().find(|b| b.action == action).copied();
    let x = on(Action::HeadLookX).expect("the mouse looks sideways");
    let y = on(Action::HeadLookY).expect("the mouse looks up and down");
    assert_eq!(x.source, Source::MouseMotion { y: false, gate: Gate::Look });
    assert_eq!(y.source, Source::MouseMotion { y: true, gate: Gate::Look });
    assert!(x.scale > 0.0 && y.scale < 0.0 && (x.scale + y.scale).abs() < 1e-9, "{} {}", x.scale, y.scale);
    assert!(!x.per_second && !y.per_second, "the speed is read from the frame's pixels and time");
    // The text of every look binding is read back as the same binding, in a config file too.
    let bindings = Bindings::default();
    for b in bindings.0.iter().filter(|b| matches!(b.action, Action::HeadLookX | Action::HeadLookY)) {
        let text = super::source::encode(b);
        assert_eq!(super::source::parse(b.action, &text).as_ref(), Ok(b), "{text}");
    }
    let saved = bindings.merge_config("").unwrap();
    let mut loaded = Bindings::default();
    loaded.0.retain(|b| !matches!(b.action, Action::HeadLookX | Action::HeadLookY));
    loaded.apply_config(&saved, "test");
    let of = |b: &Bindings, a| b.0.iter().filter(|x| x.action == a).copied().collect::<Vec<_>>();
    for action in [Action::HeadLookX, Action::HeadLookY] {
        assert_eq!(of(&loaded, action), of(&bindings, action), "{}", action.name());
    }
    // A player can drop the mouse look, or move it, as any other binding.
    let mut custom = Bindings::default();
    custom.apply_config("[bindings]\nhead_look_x=['key:KeyL']\nhead_look_y=[]", "test");
    assert_eq!(of(&custom, Action::HeadLookX).len(), 1);
    assert!(of(&custom, Action::HeadLookY).is_empty());
}

/// The head after the mouse moved at `pixels` a second for `seconds`, then stood still for `rest` seconds.
fn head_after_mouse(pixels_per_second: f32, seconds: f32, rest: f32, fps: u32) -> (f32, f32) {
    let dt = 1.0 / fps as f32;
    let mut look = HeadLook::default();
    let mut state = ActionState::default();
    let bindings = Bindings::default();
    let mut peak = 0.0f32;
    for frame in 0..((seconds + rest) * fps as f32) as u32 {
        let moving = (frame as f32) < seconds * fps as f32;
        let delta = if moving { pixels_per_second * dt } else { 0.0 };
        state.update(&bindings, &mouse((delta, 0.0), dt), false);
        look.update([state.value(Action::HeadLookX), state.value(Action::HeadLookY)], dt);
        peak = peak.max(look.right_deg());
    }
    (peak, look.right_deg())
}

#[test]
fn moving_the_mouse_turns_the_head_and_it_springs_back_when_the_mouse_stops() {
    for fps in [30, 60, 144, 240] {
        // A steady sweep at the end of the stick reaches the limit and no more; stopped, it is back in two seconds.
        let (peak, end) = head_after_mouse(700.0, 3.0, 2.5, fps);
        assert!(peak > 10.9 && peak <= 11.0 + 1e-3, "{fps} fps: {peak}");
        assert!(end.abs() < 0.01, "{fps} fps: {end}");
        // Half the speed is half the turn.
        let (peak, _) = head_after_mouse(350.0, 3.0, 0.0, fps);
        assert!((peak - 5.5).abs() < 0.05, "{fps} fps: {peak}");
        // A short flick of the wrist (300 pixels in a quarter of a second) turns it a few degrees and then it returns.
        let (peak, end) = head_after_mouse(1200.0, 0.25, 2.5, fps);
        assert!(peak > 5.0 && peak < 8.0, "{fps} fps: {peak}");
        assert!(end.abs() < 0.01, "{fps} fps: {end}");
    }
}

#[test]
fn the_amount_a_nudge_of_the_mouse_turns_the_head_does_not_depend_on_the_frame_rate() {
    // 2 pixels, moved in one frame, at any rate: about the same total push (the integral of the stick over time), and the
    // stick is never at its end, so nothing is held back.
    let peaks: Vec<f32> = [30, 60, 144, 240]
        .into_iter()
        .map(|fps| {
            let dt = 1.0 / fps as f32;
            let mut look = HeadLook::default();
            let mut state = ActionState::default();
            let bindings = Bindings::default();
            let mut peak = 0.0f32;
            for frame in 0..fps {
                let delta = if frame == 0 { 2.0 } else { 0.0 };
                state.update(&bindings, &mouse((delta, 0.0), dt), false);
                look.update([state.value(Action::HeadLookX), 0.0], dt);
                peak = peak.max(look.right_deg());
            }
            peak
        })
        .collect();
    let (low, high) = peaks.iter().fold((f32::MAX, 0.0f32), |(l, h), p| (l.min(*p), h.max(*p)));
    assert!(low > 0.1 && high < 0.13 && high - low < 0.01, "{peaks:?}");
}

#[test]
fn a_look_axis_never_leaves_the_stick_range_however_many_devices_push_it() {
    let mut snap = mouse((900.0, -900.0), 0.01);
    snap.pad_axes.insert(GamepadAxis::RightStickX, 1.0);
    snap.keys.extend([KeyCode::Numpad6, KeyCode::Numpad9]);
    assert_eq!(axes(&snap), [1.0, 1.0]);
}
