use super::state::{Button, Direction, Nav, PadInput, PadNav, press};

fn held(buttons: &[Button]) -> PadInput {
    PadInput { held: buttons.iter().copied().collect(), stick: (0.0, 0.0) }
}

fn stick(x: f32, y: f32) -> PadInput {
    PadInput { stick: (x, y), ..PadInput::default() }
}

#[test]
fn the_chord_needs_both_stick_clicks_and_fires_once() {
    let mut nav = PadNav::default();
    assert!(nav.step(&held(&[Button::LeftStick]), 0.016, false).is_empty(), "one click is nothing");
    assert_eq!(nav.step(&held(&[Button::LeftStick, Button::RightStick]), 0.016, false), [Nav::Chord]);
    assert!(nav.step(&held(&[Button::LeftStick, Button::RightStick]), 0.016, false).is_empty(), "held: once");
    nav.step(&held(&[]), 0.016, false);
    assert_eq!(
        nav.step(&held(&[Button::RightStick, Button::LeftStick]), 0.016, false),
        [Nav::Chord],
        "again after release"
    );
}

#[test]
fn the_chord_also_works_while_a_panel_is_up() {
    let mut nav = PadNav::default();
    assert_eq!(nav.step(&held(&[Button::LeftStick, Button::RightStick]), 0.016, true), [Nav::Chord]);
}

#[test]
fn with_no_panel_up_the_buttons_do_nothing() {
    let mut nav = PadNav::default();
    for button in [Button::A, Button::B, Button::Up, Button::LeftBumper] {
        assert!(nav.step(&held(&[button]), 0.016, false).is_empty(), "{button:?}");
    }
    assert!(nav.step(&stick(0.0, 1.0), 0.016, false).is_empty());
}

#[test]
fn a_button_pressed_before_the_panel_opened_does_not_act_when_it_does() {
    let mut nav = PadNav::default();
    nav.step(&held(&[Button::A]), 0.016, false);
    assert!(nav.step(&held(&[Button::A]), 0.016, true).is_empty(), "A was already down");
    nav.step(&held(&[]), 0.016, true);
    assert_eq!(nav.step(&held(&[Button::A]), 0.016, true), [Nav::Activate]);
}

#[test]
fn a_b_and_the_bumpers_act_once_per_press() {
    let mut nav = PadNav::default();
    let mut once = |button: Button, expected: Nav| {
        assert_eq!(nav.step(&held(&[button]), 0.016, true), [expected], "{button:?}");
        assert!(nav.step(&held(&[button]), 0.016, true).is_empty(), "{button:?} held");
        nav.step(&held(&[]), 0.016, true);
    };
    once(Button::A, Nav::Activate);
    once(Button::B, Nav::Back);
    once(Button::LeftBumper, Nav::Tab(false));
    once(Button::RightBumper, Nav::Tab(true));
}

#[test]
fn a_direction_moves_once_then_repeats_after_a_delay() {
    let mut nav = PadNav::default();
    let down = [Nav::Arrow(Direction::Down)];
    assert_eq!(nav.step(&held(&[Button::Down]), 0.016, true), down);
    // Held for 0.3 s: still inside the first delay.
    for _ in 0..18 {
        assert!(nav.step(&held(&[Button::Down]), 0.016, true).is_empty());
    }
    // It repeats once the delay has run out, and then at the repeat rate.
    let mut moves = 0;
    for _ in 0..60 {
        moves += nav.step(&held(&[Button::Down]), 0.016, true).len();
    }
    assert!((6..=9).contains(&moves), "{moves} repeats in about a second");
    nav.step(&held(&[]), 0.016, true);
    assert_eq!(nav.step(&held(&[Button::Down]), 0.016, true), down, "a new press moves at once");
}

#[test]
fn the_stick_moves_like_the_d_pad_with_a_hysteresis() {
    let mut nav = PadNav::default();
    assert!(nav.step(&stick(0.0, 0.4), 0.016, true).is_empty(), "a small push is nothing");
    assert_eq!(nav.step(&stick(0.0, 0.9), 0.016, true), [Nav::Arrow(Direction::Up)]);
    assert!(nav.step(&stick(0.0, 0.45), 0.016, true).is_empty(), "still held above the lower limit");
    nav.step(&stick(0.0, 0.1), 0.016, true);
    assert_eq!(nav.step(&stick(-0.9, 0.0), 0.016, true), [Nav::Arrow(Direction::Left)]);
    nav.step(&stick(0.0, 0.0), 0.016, true);
    assert_eq!(nav.step(&stick(0.0, -0.9), 0.016, true), [Nav::Arrow(Direction::Down)]);
    nav.step(&stick(0.0, 0.0), 0.016, true);
    assert_eq!(nav.step(&stick(0.9, 0.0), 0.016, true), [Nav::Arrow(Direction::Right)]);
}

#[test]
fn the_test_hook_presses_mean_what_the_buttons_do() {
    assert_eq!(press(Button::A), Some(Nav::Activate));
    assert_eq!(press(Button::B), Some(Nav::Back));
    assert_eq!(press(Button::Left), Some(Nav::Arrow(Direction::Left)));
    assert_eq!(press(Button::LeftStick), None);
    assert_eq!(Button::of_word("rb"), Some(Button::RightBumper));
    assert_eq!(Button::of_word("x"), None);
}
