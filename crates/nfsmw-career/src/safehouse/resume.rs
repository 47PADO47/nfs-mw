//! Leaving the safe house: Resume Free Roam and the roaming restart (`docs/specs/career-safehouse-2.md` sections
//! 6 and 7).

use super::context::FrontEndContext;

/// The prologue events in the order the game tries them: the first one that is not complete is started.
pub const PROLOGUE_EVENTS: [&str; 5] = ["16.1.0", "16.2.2", "16.2.3", "16.1.1", "16.2.1"];
/// The event whose completion ends the prologue.
pub const PROLOGUE_END_EVENT: &str = "16.2.1";
/// The mode a pending prologue node is made pending with.
pub const PENDING_NODE_MODE: u8 = 2;

/// The completion state of the career graph nodes (bit 1 of the node state flags), asked by node id.
pub trait EventProgress {
    /// True when the event node is complete.
    fn is_complete(&self, event: &str) -> bool;
}

impl<F: Fn(&str) -> bool> EventProgress for F {
    fn is_complete(&self, event: &str) -> bool {
        self(event)
    }
}

/// An event made pending before the world loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingEvent {
    /// The node id (for example `16.1.0`).
    pub node: &'static str,
    /// The mode, [`PENDING_NODE_MODE`].
    pub mode: u8,
}

/// The effects of starting free roam (`SetRoaming`, 7.3), as far as they matter to the career state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoamingRestart {
    /// Post `EReloadHud`: not when the previous node was `16.2.1`, and not when the manager byte `+0x2D4` is set.
    pub reload_hud: bool,
    /// Post `EAutoSave` (always; the autosave gate still decides, see [`crate::savefile::autosave`]).
    pub request_autosave: bool,
    /// The rain override is cleared.
    pub clear_rain: bool,
}

impl RoamingRestart {
    /// The effects for a restart. `previous_node_was_prologue_end` is whether the node that was current when
    /// `SetRoaming` ran was `16.2.1`; `manager_byte_2d4` is the manager flag that suppresses the HUD reload.
    pub const fn new(previous_node_was_prologue_end: bool, manager_byte_2d4: bool) -> RoamingRestart {
        RoamingRestart {
            reload_hud: !previous_node_was_prologue_end && !manager_byte_2d4,
            request_autosave: true,
            clear_rain: true,
        }
    }
}

/// What Resume Free Roam decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeOutcome {
    /// The prologue is unfinished: this node is made pending and activated; the world loads straight into the
    /// event (its own start position, no free roam, no autosave).
    PendingEvent(PendingEvent),
    /// The world loads as free roam: no pending node, `SetRoaming` runs at the end of the race status constructor.
    FreeRoam(RoamingRestart),
}

/// The accept message of the Resume Free Roam confirmation:
///
/// - with the debug byte set the whole prologue check is skipped;
/// - when `16.2.1` is complete nothing is selected and the world loads as free roam;
/// - otherwise the first of `16.1.0`, `16.2.2`, `16.2.3`, `16.1.1`, `16.2.1` that is not complete becomes the
///   pending node, with mode 2.
///
/// In every case the front end is left and context bit 0 is set (`context` is updated). Nothing is sent to the
/// scripts and heat, the respawn fields (apart from the flag the opening request set), the car and the cops are
/// not touched. In free roam `SetRoaming` runs with no previous node, so the HUD reload is posted
/// ([`RoamingRestart`]).
pub fn resume_free_roam(
    progress: &impl EventProgress,
    debug_skip_prologue: bool,
    context: &mut FrontEndContext,
) -> ResumeOutcome {
    context.insert(FrontEndContext::CAREER);
    if debug_skip_prologue || progress.is_complete(PROLOGUE_END_EVENT) {
        return ResumeOutcome::FreeRoam(RoamingRestart::new(false, false));
    }
    let node = PROLOGUE_EVENTS.iter().copied().find(|event| !progress.is_complete(event)).unwrap_or(PROLOGUE_END_EVENT);
    ResumeOutcome::PendingEvent(PendingEvent { node, mode: PENDING_NODE_MODE })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn done<'a>(events: &'a [&'a str]) -> impl Fn(&str) -> bool + 'a {
        move |event| events.contains(&event)
    }

    fn pending(node: &'static str) -> ResumeOutcome {
        ResumeOutcome::PendingEvent(PendingEvent { node, mode: 2 })
    }

    #[test]
    fn a_fresh_prologue_starts_the_opening_race() {
        let mut ctx = FrontEndContext(0);
        assert_eq!(resume_free_roam(&done(&[]), false, &mut ctx), pending("16.1.0"));
        assert!(ctx.contains(FrontEndContext::CAREER));
    }

    #[test]
    fn the_first_unfinished_event_in_table_order_is_chosen() {
        let mut ctx = FrontEndContext(0);
        assert_eq!(resume_free_roam(&done(&["16.1.0"]), false, &mut ctx), pending("16.2.2"));
        assert_eq!(resume_free_roam(&done(&["16.1.0", "16.2.2"]), false, &mut ctx), pending("16.2.3"));
        assert_eq!(resume_free_roam(&done(&["16.1.0", "16.2.2", "16.2.3"]), false, &mut ctx), pending("16.1.1"));
        assert_eq!(
            resume_free_roam(&done(&["16.1.0", "16.2.2", "16.2.3", "16.1.1"]), false, &mut ctx),
            pending("16.2.1")
        );
    }

    #[test]
    fn the_table_order_wins_over_completion_order() {
        let mut ctx = FrontEndContext(0);
        assert_eq!(resume_free_roam(&done(&["16.2.3", "16.1.1"]), false, &mut ctx), pending("16.1.0"));
        assert_eq!(resume_free_roam(&done(&["16.1.0", "16.2.3"]), false, &mut ctx), pending("16.2.2"));
    }

    #[test]
    fn a_finished_prologue_loads_free_roam_with_the_restart_effects() {
        let mut ctx = FrontEndContext(0);
        let out = resume_free_roam(&done(&["16.2.1"]), false, &mut ctx);
        assert_eq!(
            out,
            ResumeOutcome::FreeRoam(RoamingRestart { reload_hud: true, request_autosave: true, clear_rain: true })
        );
    }

    #[test]
    fn the_debug_byte_skips_the_prologue_check() {
        let mut ctx = FrontEndContext(0);
        assert!(matches!(resume_free_roam(&done(&[]), true, &mut ctx), ResumeOutcome::FreeRoam(_)));
    }

    #[test]
    fn the_hud_reload_has_two_exceptions_and_autosave_and_rain_have_none() {
        assert!(RoamingRestart::new(false, false).reload_hud);
        assert!(!RoamingRestart::new(true, false).reload_hud);
        assert!(!RoamingRestart::new(false, true).reload_hud);
        for (a, b) in [(false, false), (true, false), (false, true), (true, true)] {
            let r = RoamingRestart::new(a, b);
            assert!(r.request_autosave && r.clear_rain);
        }
    }

    #[test]
    fn the_context_keeps_its_other_bits() {
        let mut ctx = FrontEndContext(FrontEndContext::RAP_SHEET.0);
        resume_free_roam(&done(&["16.2.1"]), false, &mut ctx);
        assert!(ctx.contains(FrontEndContext::RAP_SHEET) && ctx.contains(FrontEndContext::CAREER));
    }
}
