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
fn left_or_right_held_long_jumps_but_up_and_down_never_do() {
    for (button, jumps) in [(Button::Right, true), (Button::Down, false)] {
        let mut nav = PadNav::default();
        let mut jumped = 0;
        for _ in 0..150 {
            jumped += nav.step(&held(&[button]), 0.016, true).iter().filter(|nav| matches!(nav, Nav::Jump(_))).count();
        }
        assert_eq!(jumped > 0, jumps, "{button:?} held for 2.4 s");
    }
    let mut nav = PadNav::default();
    let mut all = Vec::new();
    for _ in 0..150 {
        all.extend(nav.step(&held(&[Button::Left]), 0.016, true));
    }
    assert_eq!(all.first(), Some(&Nav::Arrow(Direction::Left)), "a press first steps once");
    assert_eq!(all.last(), Some(&Nav::Jump(false)), "then jumps back");
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

/// One egui pass over the devtools menu with the events the controller made, as the UI pass runs it.
fn menu_pass(ctx: &egui::Context, events: Vec<egui::Event>) {
    use crate::devtools::menu::{MenuView, show};
    use crate::devtools::metrics::ShowMetrics;
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0))),
        events,
        focused: true,
        ..Default::default()
    };
    ctx.begin_pass(raw);
    show(ctx, &MenuView { garage: true, metrics: ShowMetrics::Off, recording: false, scenario: None, pad: false });
    ctx.end_pass().textures_delta.clear();
}

fn events_of(nav: Nav, ctx: &egui::Context) -> Vec<egui::Event> {
    let focused = ctx.memory(|memory| memory.focused().is_some());
    let mut events = Vec::new();
    super::system::translate(nav, focused, false, &mut events);
    events
}

#[test]
fn the_pad_walks_the_devtools_menu_through_egui_focus() {
    let ctx = egui::Context::default();
    menu_pass(&ctx, Vec::new());
    assert!(ctx.memory(|memory| memory.focused()).is_none(), "nothing has the focus before the pad is used");
    // The first move gives the focus to the first line (egui moves a focus, it does not start one).
    let events = events_of(Nav::Arrow(Direction::Down), &ctx);
    menu_pass(&ctx, events);
    menu_pass(&ctx, Vec::new());
    let first = ctx.memory(|memory| memory.focused()).expect("the first line has the focus");
    // The next moves go down the list.
    let events = events_of(Nav::Arrow(Direction::Down), &ctx);
    menu_pass(&ctx, events);
    menu_pass(&ctx, Vec::new());
    let second = ctx.memory(|memory| memory.focused()).expect("the second line has the focus");
    assert_ne!(first, second);
    let events = events_of(Nav::Arrow(Direction::Up), &ctx);
    menu_pass(&ctx, events);
    menu_pass(&ctx, Vec::new());
    assert_eq!(ctx.memory(|memory| memory.focused()), Some(first), "and back up");
}
