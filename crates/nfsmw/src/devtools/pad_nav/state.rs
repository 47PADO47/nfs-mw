//! What a controller means for the developer panels, as plain data: buttons and a stick in, navigation out.

use std::collections::HashSet;

/// A controller button the panels read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    LeftBumper,
    RightBumper,
    LeftStick,
    RightStick,
}

impl Button {
    /// The word the `devtools press` console command takes, for tests and screenshots.
    pub(crate) fn of_word(word: &str) -> Option<Self> {
        Some(match word {
            "up" => Self::Up,
            "down" => Self::Down,
            "left" => Self::Left,
            "right" => Self::Right,
            "a" => Self::A,
            "b" => Self::B,
            "lb" => Self::LeftBumper,
            "rb" => Self::RightBumper,
            _ => return None,
        })
    }
}

/// What the controller asks of the panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Nav {
    /// Move the focus (or, on a slider or a stepper, change it): an arrow key.
    Arrow(Direction),
    /// Press the focused widget: Enter.
    Activate,
    /// Back: close the popup or leave the text field, else the panel.
    Back,
    /// The focus to the next widget (`false`: the previous one).
    Tab(bool),
    /// The chord that opens and closes the devtools menu.
    Chord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    const ALL: [Self; 4] = [Self::Up, Self::Down, Self::Left, Self::Right];

    fn button(self) -> Button {
        match self {
            Self::Up => Button::Up,
            Self::Down => Button::Down,
            Self::Left => Button::Left,
            Self::Right => Button::Right,
        }
    }

    /// The stick's push along this direction (`x` right, `y` up).
    fn push(self, stick: (f32, f32)) -> f32 {
        match self {
            Self::Up => stick.1,
            Self::Down => -stick.1,
            Self::Left => -stick.0,
            Self::Right => stick.0,
        }
    }
}

/// What the controller is doing now.
#[derive(Clone, Debug, Default)]
pub(crate) struct PadInput {
    pub held: HashSet<Button>,
    /// The left stick: `x` right, `y` up, -1 to 1.
    pub stick: (f32, f32),
}

/// A stick pushed this far counts as a press, and until it falls below the lower one it is held.
const PRESS: f32 = 0.55;
const RELEASE: f32 = 0.35;
/// Seconds a direction is held before it repeats, and between repeats.
const FIRST_REPEAT: f32 = 0.4;
const REPEAT: f32 = 0.12;

/// The controller's state between frames: what was held, and how long each direction has been.
#[derive(Debug, Default)]
pub(crate) struct PadNav {
    before: HashSet<Button>,
    /// For each of the four directions: held, and the seconds to its next repeat.
    directions: [Option<f32>; 4],
    chord: bool,
}

impl PadNav {
    /// One frame of `dt` seconds. With no panel up only the chord is read. A direction repeats while held, a button acts once.
    pub(crate) fn step(&mut self, input: &PadInput, dt: f32, panel_up: bool) -> Vec<Nav> {
        let mut out = Vec::new();
        let both = input.held.contains(&Button::LeftStick) && input.held.contains(&Button::RightStick);
        if both && !self.chord {
            out.push(Nav::Chord);
        }
        self.chord = both;
        if !panel_up {
            self.before = input.held.clone();
            self.directions = [None; 4];
            return out;
        }
        for (at, direction) in Direction::ALL.into_iter().enumerate() {
            let held = self.directions[at].is_some();
            let limit = match held {
                true => RELEASE,
                false => PRESS,
            };
            let down = input.held.contains(&direction.button()) || direction.push(input.stick) >= limit;
            if !down {
                self.directions[at] = None;
                continue;
            }
            let Some(left) = self.directions[at] else {
                self.directions[at] = Some(FIRST_REPEAT);
                out.push(Nav::Arrow(direction));
                continue;
            };
            let left = left - dt;
            if left > 0.0 {
                self.directions[at] = Some(left);
                continue;
            }
            self.directions[at] = Some(left + REPEAT);
            out.push(Nav::Arrow(direction));
        }
        let pressed = |button: Button| input.held.contains(&button) && !self.before.contains(&button);
        if pressed(Button::A) {
            out.push(Nav::Activate);
        }
        if pressed(Button::B) {
            out.push(Nav::Back);
        }
        if pressed(Button::LeftBumper) {
            out.push(Nav::Tab(false));
        }
        if pressed(Button::RightBumper) {
            out.push(Nav::Tab(true));
        }
        self.before = input.held.clone();
        out
    }
}

/// What a button pressed once means (the `devtools press` hook).
pub(crate) fn press(button: Button) -> Option<Nav> {
    Some(match button {
        Button::Up => Nav::Arrow(Direction::Up),
        Button::Down => Nav::Arrow(Direction::Down),
        Button::Left => Nav::Arrow(Direction::Left),
        Button::Right => Nav::Arrow(Direction::Right),
        Button::A => Nav::Activate,
        Button::B => Nav::Back,
        Button::LeftBumper => Nav::Tab(false),
        Button::RightBumper => Nav::Tab(true),
        Button::LeftStick | Button::RightStick => return None,
    })
}
