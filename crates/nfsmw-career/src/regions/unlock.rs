//! The region unlock screen `SafehouseRegionUnlock.fng` (`docs/specs/career-regions.md` section 7).
//!
//! It is step 2 of the Blacklist arrival stepper and only appears when the stage byte is exactly 12 or 8. It changes
//! no game state: it shows a text over the background picture of the rival just beaten and waits for the player.

/// FEng ticks per second on PC.
pub const FENG_TICKS_PER_SECOND: u32 = 1200;
/// The exit script after Accept: 600 ticks.
pub const LEAVE_TICKS: u32 = 600;
/// The leave delay in milliseconds (0.5 s).
pub const LEAVE_MS: u32 = LEAVE_TICKS * 1000 / FENG_TICKS_PER_SECOND;
/// The click sound posted when Accept is pressed (`SOUND_ACCEPT`).
pub const SOUND_ACCEPT: u32 = 0x7B6B_89D7;

/// What the screen shows at a stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionPicture {
    /// The language table label of the text (`REGION_UNLOCK_COSTAL` or `REGION_UNLOCK_DOWNTOWN`).
    pub label: &'static str,
    /// The label hash of the text.
    pub label_hash: u32,
    /// The rival picture key (`stage + 1`): 13 or 9.
    pub picture_key: u8,
}

impl RegionPicture {
    /// The background texture name `RIVAL_<key>_BG` (two digits, zero padded; 512 x 512 in `Global\Rivals.bin`).
    pub fn texture_name(&self) -> String {
        format!("RIVAL_{:02}_BG", self.picture_key)
    }
}

/// The screen content for a stage: stage 12 shows `REGION_UNLOCK_COSTAL` over `RIVAL_13_BG`, stage 8 shows
/// `REGION_UNLOCK_DOWNTOWN` over `RIVAL_09_BG`, every other stage has no screen (the stepper skips it).
pub const fn region_unlock_screen(stage: u8) -> Option<RegionPicture> {
    match stage {
        12 => Some(RegionPicture { label: "REGION_UNLOCK_COSTAL", label_hash: 0x29E4_B193, picture_key: 13 }),
        8 => Some(RegionPicture { label: "REGION_UNLOCK_DOWNTOWN", label_hash: 0x2B0B_CA2D, picture_key: 9 }),
        _ => None,
    }
}

/// The inputs the screen reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockInput {
    /// The pad accept button or the Enter key.
    Accept,
    /// A mouse release on the Continue button, which has the same response as Accept.
    MouseReleaseOnContinue,
    /// A mouse release anywhere else: nothing.
    MouseReleaseElsewhere,
    /// Back: nothing.
    Back,
    /// Start: nothing.
    Start,
}

/// The state of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockState {
    /// Waiting for the player. Input is live at once (the intro keeps playing) and nothing times out.
    Waiting,
    /// Accept was pressed: the out scripts run and the button is locked; `elapsed` ticks of the 600.
    Leaving {
        /// Ticks since Accept.
        elapsed: u32,
    },
    /// The leave finished.
    Finished,
}

/// What the screen did in response to an input or a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockEvent {
    /// Nothing.
    None,
    /// Accept accepted: the click sound plays, every object switches to its out script and the button locks.
    Accepted {
        /// The click sound id.
        sound: u32,
    },
    /// The 600-tick script reached its end: `EXIT_COMPLETE` is posted and `PAD_ACCEPT` goes to the game, which
    /// runs the arrival stepper (the next step: the autosave flow if possible, else the rival bio).
    Done,
}

/// The tiny state machine of the screen: accept or a Continue click only, a 0.5 s leave, back ignored, no timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnlockScreen {
    /// The state.
    pub state: UnlockState,
}

impl UnlockScreen {
    /// A fresh screen, waiting.
    pub const fn new() -> UnlockScreen {
        UnlockScreen { state: UnlockState::Waiting }
    }

    /// One input. Accept and a Continue click start the leave once; everything else (Back, Start, a click
    /// elsewhere, any input while leaving or finished) does nothing.
    pub fn input(&mut self, input: UnlockInput) -> UnlockEvent {
        let accepts = matches!(input, UnlockInput::Accept | UnlockInput::MouseReleaseOnContinue);
        if !accepts || self.state != UnlockState::Waiting {
            return UnlockEvent::None;
        }
        self.state = UnlockState::Leaving { elapsed: 0 };
        UnlockEvent::Accepted { sound: SOUND_ACCEPT }
    }

    /// Advances time by `ticks` FEng ticks (1200 per second). Waiting never ends by itself. Leaving finishes at
    /// tick 600.
    pub fn tick(&mut self, ticks: u32) -> UnlockEvent {
        let UnlockState::Leaving { elapsed } = self.state else { return UnlockEvent::None };
        let elapsed = elapsed.saturating_add(ticks);
        if elapsed < LEAVE_TICKS {
            self.state = UnlockState::Leaving { elapsed };
            return UnlockEvent::None;
        }
        self.state = UnlockState::Finished;
        UnlockEvent::Done
    }

    /// The same as [`UnlockScreen::tick`] with milliseconds.
    pub fn tick_ms(&mut self, ms: u32) -> UnlockEvent {
        self.tick(ms.saturating_mul(FENG_TICKS_PER_SECOND) / 1000)
    }
}

impl Default for UnlockScreen {
    fn default() -> Self {
        UnlockScreen::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safehouse::{BlacklistStepper, FrontEndContext, StepAction};
    use crate::settings::CareerSettings;

    #[test]
    fn only_stages_12_and_8_have_a_screen() {
        for stage in 0..=21u8 {
            assert_eq!(region_unlock_screen(stage).is_some(), matches!(stage, 8 | 12), "stage {stage}");
        }
    }

    #[test]
    fn the_stepper_shows_the_screen_exactly_when_this_function_says_so() {
        for stage in 1..=16u8 {
            let mut settings = CareerSettings::new_career();
            settings.stage = stage;
            let mut ctx = FrontEndContext::CAREER;
            let mut stepper = BlacklistStepper::new();
            stepper.advance(&mut settings, &mut ctx, false);
            stepper.advance(&mut settings, &mut ctx, false);
            let third = stepper.advance(&mut settings, &mut ctx, false);
            assert_eq!(third == StepAction::RegionUnlock, region_unlock_screen(stage).is_some(), "stage {stage}");
        }
    }

    #[test]
    fn stage_12_shows_the_coastal_text_over_rival_13() {
        let p = region_unlock_screen(12).unwrap();
        assert_eq!((p.label, p.label_hash, p.picture_key), ("REGION_UNLOCK_COSTAL", 0x29E4B193, 13));
        assert_eq!(p.texture_name(), "RIVAL_13_BG");
    }

    #[test]
    fn stage_8_shows_the_downtown_text_over_rival_09() {
        let p = region_unlock_screen(8).unwrap();
        assert_eq!((p.label, p.label_hash, p.picture_key), ("REGION_UNLOCK_DOWNTOWN", 0x2B0BCA2D, 9));
        assert_eq!(p.texture_name(), "RIVAL_09_BG", "two digits, zero padded");
    }

    #[test]
    fn the_picture_key_is_the_stage_plus_one() {
        for stage in [8u8, 12] {
            assert_eq!(region_unlock_screen(stage).unwrap().picture_key, stage + 1);
        }
    }

    #[test]
    fn the_leave_takes_half_a_second() {
        assert_eq!(LEAVE_MS, 500);
        assert_eq!(FENG_TICKS_PER_SECOND, 1200);
    }

    #[test]
    fn accept_starts_the_leave_and_the_screen_finishes_at_600_ticks() {
        let mut s = UnlockScreen::new();
        assert_eq!(s.input(UnlockInput::Accept), UnlockEvent::Accepted { sound: SOUND_ACCEPT });
        assert_eq!(s.tick(599), UnlockEvent::None);
        assert_eq!(s.tick(1), UnlockEvent::Done);
        assert_eq!(s.state, UnlockState::Finished);
        assert_eq!(s.tick(10_000), UnlockEvent::None, "done only once");
    }

    #[test]
    fn a_continue_click_has_the_same_response_as_accept() {
        let mut s = UnlockScreen::new();
        assert_eq!(s.input(UnlockInput::MouseReleaseOnContinue), UnlockEvent::Accepted { sound: SOUND_ACCEPT });
        assert_eq!(s.tick_ms(500), UnlockEvent::Done);
    }

    #[test]
    fn back_start_and_clicks_elsewhere_do_nothing() {
        let mut s = UnlockScreen::new();
        for input in [UnlockInput::Back, UnlockInput::Start, UnlockInput::MouseReleaseElsewhere] {
            assert_eq!(s.input(input), UnlockEvent::None);
            assert_eq!(s.state, UnlockState::Waiting);
        }
    }

    #[test]
    fn nothing_times_out() {
        let mut s = UnlockScreen::new();
        for _ in 0..1000 {
            assert_eq!(s.tick(u32::MAX / 2), UnlockEvent::None);
        }
        assert_eq!(s.state, UnlockState::Waiting);
    }

    #[test]
    fn the_button_is_locked_while_leaving() {
        let mut s = UnlockScreen::new();
        s.input(UnlockInput::Accept);
        s.tick(300);
        assert_eq!(s.input(UnlockInput::Accept), UnlockEvent::None, "no second click sound");
        assert_eq!(s.state, UnlockState::Leaving { elapsed: 300 });
        assert_eq!(s.tick(300), UnlockEvent::Done);
        assert_eq!(s.input(UnlockInput::Accept), UnlockEvent::None);
    }

    #[test]
    fn millisecond_ticks_convert_at_1200_per_second() {
        let mut s = UnlockScreen::new();
        s.input(UnlockInput::Accept);
        assert_eq!(s.tick_ms(499), UnlockEvent::None, "598 ticks");
        assert_eq!(s.tick_ms(2), UnlockEvent::Done);
    }
}
