//! Split screen, the Press Start screens (`PressStart.fng`, the original's `uiQRPressStart`): "Player N press Start". The
//! device that presses Start (or accept) is seated as that player's controller; player 2 may not take player 1's. The
//! screen for player 1 comes after the options (or the track list), then player 1's car select; the screen for player 2
//! follows it. Back from player 1 returns to the options (the track list for a drag, sprint or speed trap race, which have
//! none) and from player 2 to player 1's car select. Spec: `docs/specs/split-screen.md` section 2.
//!
//! Co-op (`Args::coop`, opened from the career menu) is the same screen for player 2 alone: player 1 is seated with it
//! (`PressStart::seat`), its car select is the career's, and Back returns to the career menu. Spec:
//! `docs/specs/coop-career.md` section 3.1.

use blackbox_feng::ids::PAD_BACK;

use super::{split, state};
use crate::frontend::ids::{self, screen};
use crate::frontend::logic::{Args, Command, Cx, Footer, ScreenLogic, Sub};
use crate::input::PlayerDevice;

/// The package's headline string (`Player %d press START.` goes here) and its body text object, by name hash.
const HEADLINE: u32 = 0xB244_CF71;
const BODY: u32 = 0x5455_70C6;
/// `Player %d press START.` and the explanation of the seating.
const PRESS_START: u32 = 0xCF53_8E1C;
const EXPLANATION: u32 = 0xA065_EFFE;

pub struct PressStart {
    /// The player this screen seats, 0 or 1.
    player: usize,
    /// Co-op: the career menu's Press Start screens, player 1's then player 2's.
    coop: bool,
    /// A device was seated: the package plays its leave and `EXIT_COMPLETE` opens the car select.
    leaving: bool,
}

impl PressStart {
    pub fn new(args: Args) -> Self {
        Self { player: usize::from(args.player.min(1)), coop: args.coop, leaving: false }
    }

    /// The headline with the player's number (the original formats it with `iPlayerNum + 1`).
    fn headline(player: usize) -> Option<String> {
        let template = crate::ui::Text::Game(PRESS_START).resolve()?;
        Some(blackbox_text::printf::fill(&template, &[(player + 1).to_string().as_str()]))
    }

    /// Back: in co-op player 2's goes to player 1's Press Start and player 1's to the career menu, with co-op off; in split
    /// screen player 2's goes to player 1's car select, player 1's to the options or the track list.
    fn back(&self, cx: &mut Cx) {
        if self.coop && self.player == 1 {
            state::with(|s| s.split.clear_seats());
            let args = Args { player: 0, coop: true, ..Args::default() };
            return cx.send(Command::Switch(screen::PRESS_START.into(), args));
        }
        if self.coop {
            state::with(|s| s.split.clear_seats());
            return cx
                .send(Command::Switch(screen::MAIN_MENU_SUB.into(), Args { sub: Sub::Career, ..Args::default() }));
        }
        if self.player == 1 {
            state::with(|s| s.split.unseat(1));
            let args = Args { custom: true, player: 0, ..Args::default() };
            return cx.send(Command::Switch(screen::CAR_SELECT.into(), args));
        }
        state::with(|s| s.split.clear_seats());
        let screen = match state::with(|s| split::skips_options(s.mode)) {
            true => screen::TRACK_SELECT,
            false => screen::TRACK_OPTIONS,
        };
        cx.send(Command::Switch(screen.into(), Args::default()));
    }

    /// The screen after the seat: in co-op player 2's Press Start after player 1's, then the career's car select for
    /// player 2 (`docs/specs/coop-career.md` section 3.1); in split screen the custom race's car select for the player.
    fn next(&self, cx: &mut Cx) {
        let (screen, args) = match (self.coop, self.player) {
            (true, 0) => (screen::PRESS_START, Args { player: 1, coop: true, ..Args::default() }),
            (true, _) => (screen::CAR_SELECT, Args { coop: true, player: 1, ..Args::default() }),
            (false, player) => (screen::CAR_SELECT, Args { custom: true, player: player as u8, ..Args::default() }),
        };
        cx.send(Command::Switch(screen.into(), args));
    }
}

impl ScreenLogic for PressStart {
    fn start(&mut self, cx: &mut Cx) {
        let headline = Self::headline(self.player);
        if let (Some(text), Some(o)) = (headline, cx.object(HEADLINE)) {
            cx.rt.set_text(o, text);
        }
        if let Some(text) = crate::ui::Text::Game(EXPLANATION).resolve()
            && let Some(o) = cx.object(BODY)
        {
            cx.rt.set_text(o, text);
        }
    }

    /// The Start press seats the device as this screen's player; a refused device (player 1's, for player 2) does nothing.
    fn start_pressed(&mut self, cx: &mut Cx, device: PlayerDevice) {
        if self.leaving || !state::with(|s| s.split.seat(self.player, device)) {
            return;
        }
        self.leaving = true;
        cx.rt.post_to_package(cx.package, ids::LEAVE_SCREEN);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_BACK if !self.leaving => self.back(cx),
            ids::EXIT_COMPLETE if self.leaving => self.next(cx),
            _ => {}
        }
    }

    fn footer(&self) -> Footer {
        Footer::BackOnly
    }
}
