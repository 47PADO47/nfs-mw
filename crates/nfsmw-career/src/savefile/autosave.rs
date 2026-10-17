//! The autosave: triggers, the gate, the pending / armed bits, the run and the failure handling
//! (`docs/specs/career-save-screens-2.md` section 8 and the rewrite rules of section 14).
//!
//! This replaces the earlier `safehouse::autosave` model. Corrections it carries: the `EAutoSave` event is posted at
//! `SetRoaming`, at an event **start** (not at the end of a race) and when a pursuit is evaded; the profile loaded
//! byte is not part of the gate (the run tests it); the gate has a one-shot skip and an "off by error" input.

use crate::safehouse::FrontEndContext;

/// The id of the final event, which never autosaves.
pub const FINAL_EVENT: &str = "1.8.1";
/// The id of the opening race, which does not post the start-of-event autosave.
pub const OPENING_RACE: &str = "16.1.0";

/// The current event node as the gate sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentNode<'a> {
    /// The node id (for example `14.2.1`).
    pub id: &'a str,
    /// The node boolean attribute `BossRace` (hash `0xFF5EE5D6`).
    pub boss_race: bool,
}

/// The inputs of the gate besides the manager bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateInputs<'a> {
    /// `force`: the screen autosave of state `0xB0` after the gate passed, and the retry after "Yes".
    pub force: bool,
    /// The front-end context word (the online and LAN bits).
    pub context: FrontEndContext,
    /// The memory-card layer is present (always true on PC).
    pub layer_present: bool,
    /// The gameplay manager current node, if any.
    pub current_node: Option<CurrentNode<'a>>,
}

impl<'a> GateInputs<'a> {
    /// Inputs for an unforced evaluation in a career context with the layer present.
    pub fn unforced(current_node: Option<CurrentNode<'a>>) -> GateInputs<'a> {
        GateInputs { force: false, context: FrontEndContext::CAREER, layer_present: true, current_node }
    }
}

/// Why an `EAutoSave` event is posted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger<'a> {
    /// `SetRoaming`: every time free roam starts. No condition.
    RoamingStart,
    /// An event starts (`StartRace` hook).
    EventStart {
        /// The id of the started node, `None` when it does not exist.
        node: Option<&'a str>,
        /// The node carries the flag `0x8CB01ABF` (probably "race-type node"; unconfirmed).
        race_flag: bool,
    },
    /// A pursuit was evaded.
    PursuitEvaded {
        /// An event is underway (`+0x1960 != 0`): then no event is posted.
        event_underway: bool,
    },
}

impl Trigger<'_> {
    /// Whether the event is posted: roaming start always; an event start when the node exists, carries the flag
    /// and is not `16.1.0`, or when the node is `1.8.1`; a pursuit evaded when no event is underway. A boss race
    /// passes the start condition and is refused by the gate.
    pub fn posts_event(&self) -> bool {
        match *self {
            Trigger::RoamingStart => true,
            Trigger::EventStart { node, race_flag } => {
                node == Some(FINAL_EVENT) || (node.is_some_and(|n| n != OPENING_RACE) && race_flag)
            }
            Trigger::PursuitEvaded { event_underway } => !event_underway,
        }
    }
}

/// What the run decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunDecision {
    /// Nothing: the gate refused, or no profile is loaded.
    Nothing,
    /// "Switched off by error" is set: the screens open in state `0x90` and ask whether to continue (the library
    /// write is not started).
    AskContinue {
        /// The overlay icon was shown first.
        overlay: bool,
    },
    /// The write starts: the autosave run and saving bytes are set, the silent mask applies.
    Write {
        /// The overlay icon is shown (not in state `0xB0`).
        overlay: bool,
    },
}

/// The prompt a failed autosave asks (Yes / No, dialog tag `0xA`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuePrompt {
    /// `MC_AUTOSAVE_CARDLESS_PS2`: the first failure. The PS2 wording is shown on PC; a rewrite can use a PC text.
    Cardless,
    /// `MC_AUTOSAVE_NO_CARD_PS2`: asked when the "switched off by error" byte was already set.
    NoCard,
}

/// What the answer to the continue prompt does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinueOutcome {
    /// Yes: re-enable is set, the option is on, the state becomes `0xB0` and the save runs again at once with
    /// `force = true` (a second failure repeats the question).
    RetryForced,
    /// No: the option is off and the screen closes. The gate then returns false at the option test until the
    /// player turns the option on in the Options.
    Closed,
}

/// The memory-card manager bytes and the profile option that the autosave reads and writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutosaveManager {
    /// The profile autosave option (profile `+0x38`), default on.
    pub option_on: bool,
    /// `+0x1F` pending: an autosave was requested.
    pub pending: bool,
    /// `+0x2A` armed: the in-game HUD update has run since `pending` was set.
    pub armed: bool,
    /// `+0x2B` skip once: the next gate evaluation returns false and clears this byte.
    pub skip_once: bool,
    /// `+0x24` switched off by error: autosave was turned off by a failure; the next autosave asks first.
    pub off_by_error: bool,
    /// `+0x25` re-enable: the "continue" answer was Yes.
    pub re_enable: bool,
    /// `+0x1E` autosave run: a background autosave is in progress.
    pub run: bool,
    /// `+0x20` saving: an autosave write is in progress.
    pub saving: bool,
}

impl Default for AutosaveManager {
    fn default() -> Self {
        AutosaveManager {
            option_on: true,
            pending: false,
            armed: false,
            skip_once: false,
            off_by_error: false,
            re_enable: false,
            run: false,
            saving: false,
        }
    }
}

impl AutosaveManager {
    /// The gate ("autosave possible"). Order, the first failure returns false:
    ///
    /// 1. `force`: true, nothing else is tested;
    /// 2. the skip-once byte is set: clear it and return false;
    /// 3. the context has the online or LAN bit: false;
    /// 4. the layer is not present: false;
    /// 5. neither the option nor the "switched off by error" byte is set: false;
    /// 6. the current node is `1.8.1`: false;
    /// 7. the current node is a boss race: false;
    /// 8. true.
    ///
    /// The profile loaded byte is **not** part of it. A background autosave evaluates it twice and a screen
    /// autosave in the caller and in the screen state step, so the first evaluation after a resume consumes the
    /// skip-once byte.
    pub fn can_autosave(&mut self, inputs: &GateInputs) -> bool {
        if inputs.force {
            return true;
        }
        if self.skip_once {
            self.skip_once = false;
            return false;
        }
        if inputs.context.contains(FrontEndContext::ONLINE) || inputs.context.contains(FrontEndContext::LAN) {
            return false;
        }
        if !inputs.layer_present {
            return false;
        }
        if !(self.option_on || self.off_by_error) {
            return false;
        }
        !matches!(inputs.current_node, Some(node) if node.id == FINAL_EVENT || node.boss_race)
    }

    /// Sets the skip-once byte the way the resume code does: resuming into a node other than `16.1.0` while stage
    /// 16, and starting the final event `1.8.1`. Returns whether it was set.
    pub fn resume_sets_skip(&mut self, stage: u8, resumed_node: Option<&str>, starting_final_event: bool) -> bool {
        let prologue_other_node = stage == 16 && resumed_node.is_some_and(|n| n != OPENING_RACE);
        if prologue_other_node || starting_final_event {
            self.skip_once = true;
            return true;
        }
        false
    }

    /// The `EAutoSave` event is disposed: it evaluates the gate with `force = false` and, when true, sets pending.
    /// The caller checks [`Trigger::posts_event`] before posting.
    pub fn event_disposed(&mut self, inputs: &GateInputs) {
        if self.can_autosave(inputs) {
            self.pending = true;
        }
    }

    /// The in-game HUD update, once per frame while a world runs: pending is copied into armed. Nothing is armed
    /// while the front end is up, so a pending autosave waits for the world.
    pub fn hud_update(&mut self) {
        if self.pending {
            self.armed = true;
        }
    }

    /// The manager update: when pending and armed are set and the gameplay manager is idle (its notification list
    /// is empty, or no notification can be shown right now), both are cleared and the run starts with
    /// `force = false`. Returns the decision of the run.
    pub fn tick(&mut self, gameplay_idle: bool, inputs: &GateInputs, profile_loaded: bool) -> Option<RunDecision> {
        if !(self.pending && self.armed && gameplay_idle) {
            return None;
        }
        self.pending = false;
        self.armed = false;
        Some(self.start_run(inputs, profile_loaded, false))
    }

    /// The run: nothing when the gate is false or no profile is loaded. Otherwise the overlay shows (except in
    /// state `0xB0`, `in_screen_state_b0`); with "switched off by error" set the continue question is asked;
    /// else the write starts (`run` and `saving` set).
    pub fn start_run(&mut self, inputs: &GateInputs, profile_loaded: bool, in_screen_state_b0: bool) -> RunDecision {
        if !self.can_autosave(inputs) || !profile_loaded {
            return RunDecision::Nothing;
        }
        let overlay = !in_screen_state_b0;
        if self.off_by_error {
            return RunDecision::AskContinue { overlay };
        }
        self.run = true;
        self.saving = true;
        RunDecision::Write { overlay }
    }

    /// The write finished (success): `profile loaded` and the other bytes are set by
    /// [`super::state::ProfileState::save_completed`]; here the manager clears `off_by_error`, `saving` and `run`.
    pub fn write_completed(&mut self) {
        self.off_by_error = false;
        self.saving = false;
        self.run = false;
    }

    /// A background autosave failed (the only error is an open failure): the buffer is freed, the saving bytes
    /// cleared and the profile option set to 0. Returns the question to ask: [`ContinuePrompt::NoCard`] when
    /// "switched off by error" was already set, else [`ContinuePrompt::Cardless`].
    pub fn write_failed(&mut self) -> ContinuePrompt {
        let prompt = match self.off_by_error {
            true => ContinuePrompt::NoCard,
            false => ContinuePrompt::Cardless,
        };
        self.saving = false;
        self.run = false;
        self.option_on = false;
        prompt
    }

    /// The answer to the continue question. Yes: `re_enable = 1`, option on, retry with `force = true`. No: option
    /// off, the screen closes.
    pub fn answer_continue(&mut self, yes: bool) -> ContinueOutcome {
        if !yes {
            self.option_on = false;
            return ContinueOutcome::Closed;
        }
        self.re_enable = true;
        self.option_on = true;
        ContinueOutcome::RetryForced
    }

    /// The quiet-mode operation reports value 1 ("arm"): the option is stored on and "switched off by error" is
    /// cleared. This is what turns autosave on after "Yes" to the enable question, after a load with the option
    /// on and after a manual save with the option on.
    pub fn arm_quiet_mode(&mut self) {
        self.option_on = true;
        self.off_by_error = false;
    }
}

/// The three places that save with the screen up (state `0xB0`), and what makes them do it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenAutosave {
    /// Blacklist stepper step 3: the caller evaluates the gate with `force = false` and skips the
    /// screen when it is false; exit continues the stepper.
    Stepper,
    /// The main menu way back to the title: when *unsaved changes* is set and the layer is present.
    TitleExit {
        /// The profile unsaved changes byte.
        unsaved_changes: bool,
    },
    /// Leaving the customization shop ("stable" wording): the stable differs from the copy taken
    /// when the shop opened and the layer is present.
    ShopExit {
        /// The stable differs from the copy.
        stable_changed: bool,
    },
}

impl ScreenAutosave {
    /// Whether the caller opens the save screen. The stepper evaluates the gate itself (consuming skip-once); the
    /// other two need the layer present and their change condition.
    pub fn opens_screen(&self, manager: &mut AutosaveManager, gate: &GateInputs) -> bool {
        match *self {
            ScreenAutosave::Stepper => manager.can_autosave(gate),
            ScreenAutosave::TitleExit { unsaved_changes } => unsaved_changes && gate.layer_present,
            ScreenAutosave::ShopExit { stable_changed } => stable_changed && gate.layer_present,
        }
    }
}

/// What the state `0xB0` step of the save screen does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenStep {
    /// No alias is loaded: the state becomes `0x60` (create).
    Create,
    /// The gate passed: show "Saving ..." and run the save with `force = true`.
    ForcedSave,
    /// The gate failed (online, LAN, option off, final event, boss race): the state becomes `0x50` and the manual
    /// question is asked instead.
    ManualQuestion,
}

/// The state `0xB0` step: no alias loaded selects create; otherwise the gate is evaluated with `force = false`.
pub fn screen_state_b0(manager: &mut AutosaveManager, gate: &GateInputs, alias_loaded: bool) -> ScreenStep {
    if !alias_loaded {
        return ScreenStep::Create;
    }
    match manager.can_autosave(gate) {
        true => ScreenStep::ForcedSave,
        false => ScreenStep::ManualQuestion,
    }
}

#[cfg(test)]
mod tests;
