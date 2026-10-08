//! Pad input: the controller mask of the frame turned into messages for the packages that have control
//! (spec `docs/specs/feng-input.md`, sections 1 and 2).

use super::messages::Target;
use super::{PackageId, Runtime};
use crate::ids::*;
use crate::package::{ResponseKind, flags};

/// Bits of the pad mask the host passes to [`Runtime::set_pad_mask`].
pub mod pad {
    pub const UP: u32 = 1 << 0;
    pub const DOWN: u32 = 1 << 1;
    pub const LEFT: u32 = 1 << 2;
    pub const RIGHT: u32 = 1 << 3;
    pub const ACCEPT: u32 = 1 << 4;
    pub const BACK: u32 = 1 << 5;
    pub const START: u32 = 1 << 6;
    pub const LTRIGGER: u32 = 1 << 7;
    pub const RTRIGGER: u32 = 1 << 8;
    /// `BUTTON0` .. `BUTTON9` are the bits from here up.
    pub const BUTTON0: u32 = 1 << 9;
}

/// Ticks a direction is held before it repeats the first time (20 frames of 16 ticks), and after that.
const FIRST_REPEAT: u32 = 20 * 16;
const FAST_REPEAT: u32 = 120;
/// Pad bits 4..19 are buttons; the first four are the directions.
const BIT_COUNT: usize = 19;

/// What a direction case sends: `(bit a, bit b or none, message)`, in the order the engine tests them.
const DIRECTIONS: [(usize, Option<usize>, u32); 8] = [
    (0, Some(2), PAD_UPLEFT),
    (0, Some(3), PAD_UPRIGHT),
    (1, Some(2), PAD_DOWNLEFT),
    (1, Some(3), PAD_DOWNRIGHT),
    (0, None, PAD_UP),
    (2, None, PAD_LEFT),
    (1, None, PAD_DOWN),
    (3, None, PAD_RIGHT),
];

/// The message each button bit sends when pressed (bits 0..3 are directions and have none).
fn pressed_message(bit: usize) -> u32 {
    match bit {
        4 => PAD_ACCEPT,
        5 => PAD_BACK,
        6 => PAD_START,
        7 => PAD_LTRIGGER,
        8 => PAD_RTRIGGER,
        b => PAD_BUTTON0 + (b - 9) as u32,
    }
}

fn released_message(bit: usize) -> u32 {
    match bit {
        4 => PAD_ACCEPT_RELEASED,
        5 => PAD_BACK_RELEASED,
        6 => PAD_START_RELEASED,
        7 => PAD_LTRIGGER_RELEASED,
        8 => PAD_RTRIGGER_RELEASED,
        b => PAD_BUTTON_RELEASED[b - 9],
    }
}

/// The controller state the engine keeps between frames.
#[derive(Clone, Debug, Default)]
pub struct PadState {
    now: u32,
    last: u32,
    /// Ticks each bit has been held.
    held: [u32; 32],
    /// Direction cases that repeated once and now repeat fast.
    fast: u8,
    /// Bits whose held count drops after this frame, and by how much.
    decrement: [u32; BIT_COUNT],
}

impl PadState {
    fn update(&mut self, mask: u32, ticks: u32) {
        self.last = self.now;
        self.now = mask;
        for bit in 0..32 {
            let m = 1 << bit;
            if self.now & m == 0 {
                continue;
            }
            if self.last & m != 0 {
                self.held[bit] = self.held[bit].saturating_add(ticks);
            } else {
                self.held[bit] = 0;
            }
        }
    }

    fn pressed(&self, bit: usize) -> bool {
        self.now & (1 << bit) != 0 && self.last & (1 << bit) == 0
    }

    fn released(&self, bit: usize) -> bool {
        self.now & (1 << bit) == 0 && self.last & (1 << bit) != 0
    }

    fn held(&self, bit: usize) -> bool {
        self.now & (1 << bit) != 0 && self.last & (1 << bit) != 0
    }

    fn active(&self) -> bool {
        self.now | self.last != 0
    }
}

impl Runtime {
    /// Sets the pad state for the next [`Runtime::update`]: a mask of [`pad`] bits.
    pub fn set_pad_mask(&mut self, mask: u32) {
        self.pad_next = mask;
    }

    /// Whether a package takes pad input (a package has control until the host takes it away).
    pub fn set_control(&mut self, package: PackageId, control: bool) {
        if let Some(p) = self.running_mut(package) {
            p.control = control;
        }
    }

    /// With this on, a start press also counts as accept for the package.
    pub fn set_start_equals_accept(&mut self, package: PackageId, on: bool) {
        if let Some(p) = self.running_mut(package) {
            p.start_equals_accept = on;
        }
    }

    /// Processes the frame's pad state for every package with control. Called by `update` before the scripts run.
    pub(super) fn process_pads(&mut self, ticks: u32) {
        let mask = self.pad_next;
        self.pad.update(mask, ticks);
        if !self.pad.active() {
            return;
        }
        self.pad.decrement = [0; BIT_COUNT];
        for id in self.ids() {
            let Some(p) = self.running(id) else { continue };
            if p.control && p.input_enabled {
                self.pads_for_package(id);
            }
        }
        for bit in 0..BIT_COUNT {
            let d = self.pad.decrement[bit];
            self.pad.held[bit] = self.pad.held[bit].saturating_sub(d);
        }
    }

    fn usable(&self, package: PackageId) -> bool {
        self.running(package).is_some_and(|p| p.input_enabled)
    }

    fn pads_for_package(&mut self, package: PackageId) {
        for bit in 4..BIT_COUNT {
            if !self.usable(package) {
                return;
            }
            self.button_bit(package, bit);
        }
        self.direction_cases(package);
    }

    /// The current button of a package.
    fn current(&self, package: PackageId) -> Option<usize> {
        self.running(package)?.current_button
    }

    fn has_response(&self, package: PackageId, button: usize, message: u32) -> bool {
        self.running(package)
            .and_then(|p| p.def.objects.get(button))
            .is_some_and(|o| o.responses.iter().any(|r| r.message == message))
    }

    fn has_package_response(&self, package: PackageId, message: u32) -> bool {
        self.running(package).is_some_and(|p| p.def.responses_to(message).is_some())
    }

    /// Sends `message` to the button (and the sound system) if it answers it, else to the package responses
    /// (and the sound system) if the package does.
    fn send_pad_message(&mut self, package: PackageId, button: Option<usize>, message: u32) {
        if let Some(b) = button
            && self.has_response(package, b, message)
        {
            self.queue(message, None, package, Target::Object(b));
            self.queue(message, Some(b), package, Target::Sound);
            return;
        }
        if self.has_package_response(package, message) {
            self.queue(message, None, package, Target::ThisGlobal);
            self.queue(message, None, package, Target::Sound);
        }
    }

    fn remember_press(&mut self, package: PackageId, bit: usize, button: Option<usize>) {
        if let Some(p) = self.running_mut(package) {
            p.pressed_on[bit] = button;
        }
    }

    fn button_bit(&mut self, package: PackageId, bit: usize) {
        let start_as_accept = bit == 4 && self.running(package).is_some_and(|p| p.start_equals_accept);
        let pad = &self.pad;
        let pressed = pad.pressed(bit) || (start_as_accept && pad.pressed(6));
        let released = pad.released(bit) || (start_as_accept && pad.released(6));
        let held = pad.held(bit) || (start_as_accept && pad.held(6));
        if !(pressed || released || held) {
            return;
        }
        let current = self.current(package);

        if bit == 4 && pressed {
            self.remember_press(package, 4, current);
            match current.filter(|&b| self.has_response(package, b, BUTTON_PRESSED)) {
                Some(b) => {
                    self.queue(BUTTON_PRESSED, None, package, Target::Object(b));
                    self.queue(BUTTON_PRESSED, Some(b), package, Target::Sound);
                }
                None if self.has_package_response(package, PAD_ACCEPT) => {
                    self.queue(PAD_ACCEPT, None, package, Target::ThisGlobal);
                    self.queue(PAD_ACCEPT, None, package, Target::Sound);
                }
                None => {}
            }
        }
        if (bit == 7 || bit == 8) && held {
            let message = if bit == 7 { PAD_LTRIGGER_HELD } else { PAD_RTRIGGER_HELD };
            self.send_pad_message(package, current, message);
        }
        if bit != 4 && pressed {
            self.remember_press(package, bit, current);
            self.send_pad_message(package, current, pressed_message(bit));
        }
        if released {
            let mut message = released_message(bit);
            let remembered = self.running(package).and_then(|p| p.pressed_on[bit]);
            if let Some(b) = current.filter(|&b| remembered == Some(b)) {
                self.remember_press(package, bit, None);
                if bit == 4 {
                    message = BUTTON_RELEASED;
                }
                if self.has_response(package, b, message) {
                    self.queue(message, None, package, Target::Object(b));
                    self.queue(message, Some(b), package, Target::Sound);
                }
            }
            if self.has_package_response(package, message) {
                self.queue(message, None, package, Target::ThisGlobal);
                self.queue(message, None, package, Target::Sound);
            }
        }
        if !self.queue.is_empty() {
            self.process_queue();
        }
    }

    fn direction_cases(&mut self, package: PackageId) {
        for (case, &(a, b, message)) in DIRECTIONS.iter().enumerate() {
            if !self.usable(package) {
                return;
            }
            let pad = &self.pad;
            let (held_for, just_pressed) = match b {
                Some(b) => (pad.held[a].min(pad.held[b]), pad.pressed(a) && pad.pressed(b)),
                None => (pad.held[a], pad.pressed(a)),
            };
            let both_down = match b {
                Some(b) => pad.now & (1 << a) != 0 && pad.now & (1 << b) != 0,
                None => pad.now & (1 << a) != 0,
            };
            if !both_down {
                self.pad.fast &= !(1 << case);
                continue;
            }
            let repeat = if self.pad.fast & (1 << case) != 0 { FAST_REPEAT } else { FIRST_REPEAT };
            let repeating = held_for >= repeat && !just_pressed;
            if !just_pressed && !repeating {
                if held_for == 0 {
                    self.pad.fast &= !(1 << case);
                }
                continue;
            }
            if repeating {
                self.pad.fast |= 1 << case;
            }
            self.pad.decrement[a] = repeat;
            if let Some(b) = b {
                self.pad.decrement[b] = repeat;
            }
            self.fire_direction(package, message, case);
            return;
        }
    }

    /// A direction fired: tell the current button or the package, and move the focus.
    fn fire_direction(&mut self, package: PackageId, message: u32, case: usize) {
        let current = self.current(package);
        let Some(button) = current else {
            if self.has_package_response(package, message) {
                self.queue(message, None, package, Target::ThisGlobal);
                self.queue(message, None, package, Target::Sound);
            }
            return;
        };
        let (flags_of_button, answers) = {
            let o = self.running(package).and_then(|p| p.def.objects.get(button));
            let answer = o.and_then(|o| o.responses.iter().find(|r| r.message == message));
            let no_nav = answer.is_some_and(|r| r.responses.iter().any(|x| x.kind == ResponseKind::Button(0x104)));
            (o.map_or(0, |o| o.flags), answer.map(|_| no_nav))
        };
        let navigates = flags_of_button & flags::DONT_NAVIGATE == 0;
        let direction = case_direction(case);
        let new_button = match answers {
            Some(no_nav) => {
                self.queue(message, None, package, Target::Object(button));
                (navigates && !no_nav).then(|| self.button_from(package, button, direction)).flatten()
            }
            None => {
                let found = navigates.then(|| self.button_from(package, button, direction)).flatten();
                self.queue(message, None, package, Target::ThisGlobal);
                found
            }
        };
        self.queue(message, Some(button), package, Target::Sound);
        if let Some(new) = new_button {
            self.release_remembered(package, button);
            self.set_focus_index(package, Some(new), true);
        }
    }

    /// Buttons that were pressed on the old current button get their release message before the focus moves.
    fn release_remembered(&mut self, package: PackageId, button: usize) {
        for bit in 4..BIT_COUNT {
            let remembered = self.running_mut(package).and_then(|p| p.pressed_on[bit].take());
            if remembered.is_none() {
                continue;
            }
            let message = released_message(bit);
            if self.has_response(package, button, message) {
                self.queue(message, None, package, Target::Object(button));
                self.queue(message, Some(button), package, Target::Sound);
            }
        }
    }
}

/// The eight directions as indices: up, up-right, right, down-right, down, down-left, left, up-left.
fn case_direction(case: usize) -> usize {
    [7, 1, 5, 3, 0, 6, 4, 2][case]
}
