//! The autosave trigger and gate (`docs/specs/career-flow.md` section 8, `docs/specs/career-safehouse.md`
//! sections 5.3 and 7.3 of the second file).
//!
//! An `AutoSave` event only raises a "save pending" bit; the save runs on a later frame when the gameplay manager
//! has no pending notification, and only if the gate lets it. [`AutosaveState`] holds the bit.

use super::context::FrontEndContext;

/// The graph node of the opening race, which never triggers an autosave.
pub const OPENING_RACE_NODE: &str = "16.1.0";

/// Why an `AutoSave` event is posted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutosaveTrigger {
    /// `SetRoaming`: every time free roam starts.
    RoamingStart,
    /// A race ended.
    RaceEnd,
}

/// Whether the event is posted: always for a free roam start and a race end, except while the current node is the
/// opening race `16.1.0`.
pub fn trigger_posts_event(_trigger: AutosaveTrigger, current_node: Option<&str>) -> bool {
    current_node != Some(OPENING_RACE_NODE)
}

/// What the gate reads. The same test is "autosave possible" for the Blacklist sequence (step 3) and the gate of
/// the save itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutosaveInputs {
    /// The front-end context word (the online and LAN bits).
    pub context: FrontEndContext,
    /// The memory-card layer is present.
    pub card_layer: bool,
    /// A profile has been loaded.
    pub profile_loaded: bool,
    /// The profile autosave option (options `+0x14`, profile `+0x38`), default on.
    pub option_on: bool,
    /// The card manager manual-save byte.
    pub manual_save: bool,
    /// The current event node carries the boolean field `0xFF5EE5D6` (meaning unconfirmed).
    pub node_exempt: bool,
    /// The current event is the final event `1.8.1` (the game is in its final-event state).
    pub final_event: bool,
}

/// True when an autosave is possible: not online or LAN, the card layer present, a profile loaded unless the
/// manual-save flag is on, the option on or the manual-save byte set, and the current event neither exempt nor the
/// final one.
pub fn autosave_possible(inputs: &AutosaveInputs) -> bool {
    if inputs.context.contains(FrontEndContext::ONLINE) || inputs.context.contains(FrontEndContext::LAN) {
        return false;
    }
    if !inputs.card_layer {
        return false;
    }
    if !inputs.profile_loaded && !inputs.manual_save {
        return false;
    }
    if !(inputs.option_on || inputs.manual_save) {
        return false;
    }
    !(inputs.node_exempt || inputs.final_event)
}

/// The "save pending" bit of the memory-card manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AutosaveState {
    /// A save is waiting for a quiet frame.
    pub pending: bool,
}

impl AutosaveState {
    /// Handles an `AutoSave` event: raises the bit when the trigger posts it.
    pub fn request(&mut self, trigger: AutosaveTrigger, current_node: Option<&str>) {
        if trigger_posts_event(trigger, current_node) {
            self.pending = true;
        }
    }

    /// One frame: the save runs when the bit is up, the gameplay manager has no pending notification and the gate
    /// allows it. Returns whether the save runs now. The bit stays up while a notification is pending; when the
    /// gate refuses it is dropped (the original does not retry; unconfirmed).
    pub fn frame(&mut self, notifications_pending: bool, inputs: &AutosaveInputs) -> bool {
        if !self.pending || notifications_pending {
            return false;
        }
        self.pending = false;
        autosave_possible(inputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok() -> AutosaveInputs {
        AutosaveInputs {
            context: FrontEndContext::CAREER,
            card_layer: true,
            profile_loaded: true,
            option_on: true,
            manual_save: false,
            node_exempt: false,
            final_event: false,
        }
    }

    #[test]
    fn the_default_profile_autosaves() {
        assert!(autosave_possible(&ok()));
    }

    #[test]
    fn each_blocker_stops_it() {
        let online = AutosaveInputs { context: FrontEndContext(FrontEndContext::ONLINE.0 | 1), ..ok() };
        let lan = AutosaveInputs { context: FrontEndContext(FrontEndContext::LAN.0 | 1), ..ok() };
        assert!(!autosave_possible(&online) && !autosave_possible(&lan));
        assert!(!autosave_possible(&AutosaveInputs { card_layer: false, ..ok() }));
        assert!(!autosave_possible(&AutosaveInputs { option_on: false, ..ok() }));
        assert!(!autosave_possible(&AutosaveInputs { node_exempt: true, ..ok() }));
        assert!(!autosave_possible(&AutosaveInputs { final_event: true, ..ok() }));
        assert!(!autosave_possible(&AutosaveInputs { profile_loaded: false, ..ok() }));
    }

    #[test]
    fn the_manual_save_flag_overrides_the_option_and_the_missing_profile() {
        assert!(autosave_possible(&AutosaveInputs { option_on: false, manual_save: true, ..ok() }));
        assert!(autosave_possible(&AutosaveInputs { profile_loaded: false, manual_save: true, ..ok() }));
        assert!(!autosave_possible(&AutosaveInputs { manual_save: true, node_exempt: true, ..ok() }));
    }

    #[test]
    fn the_opening_race_never_posts_the_event() {
        for trigger in [AutosaveTrigger::RoamingStart, AutosaveTrigger::RaceEnd] {
            assert!(!trigger_posts_event(trigger, Some("16.1.0")));
            assert!(trigger_posts_event(trigger, Some("16.2.1")));
            assert!(trigger_posts_event(trigger, None));
        }
    }

    #[test]
    fn the_save_waits_for_a_quiet_frame_and_runs_once() {
        let mut state = AutosaveState::default();
        assert!(!state.frame(false, &ok()), "nothing requested");
        state.request(AutosaveTrigger::RoamingStart, None);
        assert!(!state.frame(true, &ok()), "a notification is pending");
        assert!(state.pending);
        assert!(state.frame(false, &ok()));
        assert!(!state.frame(false, &ok()), "once");
    }

    #[test]
    fn a_refusing_gate_drops_the_request() {
        let mut state = AutosaveState::default();
        state.request(AutosaveTrigger::RaceEnd, Some("14.1.1"));
        assert!(!state.frame(false, &AutosaveInputs { option_on: false, ..ok() }));
        assert!(!state.pending);
    }

    #[test]
    fn a_request_during_the_opening_race_is_ignored() {
        let mut state = AutosaveState::default();
        state.request(AutosaveTrigger::RaceEnd, Some("16.1.0"));
        assert!(!state.pending);
    }
}
