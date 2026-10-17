//! The phone flush conditions inside the safe house and in free roam.
//!
//! `career-safehouse.md` does not state a separate rule for the phone. It follows from `career-phone.md` 3.5: the
//! flush needs the game flow state 6 (in game) and an idle race status, and the safe house is the front end (state
//! 3, no world exists), so nothing is delivered there. Messages queued meanwhile wait without a timeout and arrive
//! on the first update of the next free roam that satisfies the other conditions. The inbox list screen can still be
//! opened from the front end; that needs no flush.

use crate::phone::FlushContext;

/// The flush conditions while the safe house front end is up: nothing allows a delivery.
pub const fn safe_house_flush_context() -> FlushContext {
    FlushContext {
        free_roam: false,
        race_idle: false,
        overlay_allowed: false,
        speech_idle: false,
        player_car_ok: false,
    }
}

/// The flush conditions in free roam: the game is in the world and no event is underway (a free roam race status
/// has state 0, and `SetRoaming` has run). The other three come from the caller.
pub const fn free_roam_flush_context(overlay_allowed: bool, speech_idle: bool, player_car_ok: bool) -> FlushContext {
    FlushContext { free_roam: true, race_idle: true, overlay_allowed, speech_idle, player_car_ok }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phone::{AddOutcome, PhoneQueue, is_known};
    use crate::settings::SmsInbox;

    #[test]
    fn nothing_is_delivered_in_the_safe_house() {
        assert!(!safe_house_flush_context().can_show());
    }

    #[test]
    fn a_message_queued_in_the_safe_house_arrives_in_the_next_free_roam() {
        let mut inbox = SmsInbox::new();
        inbox.invalidate_missing(|i| is_known(u32::from(i)));
        let mut queue = PhoneQueue::new();
        assert_eq!(queue.add_sms(&mut inbox, 123), AddOutcome::Queued);
        assert_eq!(queue.flush(&mut inbox, &safe_house_flush_context()), None);
        assert_eq!(queue.pending(), &[123]);
        let busy = free_roam_flush_context(true, true, false);
        assert_eq!(queue.flush(&mut inbox, &busy), None, "the car is not ready yet");
        let out = queue.flush(&mut inbox, &free_roam_flush_context(true, true, true)).unwrap();
        assert_eq!(out.open_at, 123);
        assert_eq!(inbox.slots[123].sequence, 1);
    }

    #[test]
    fn free_roam_contexts_depend_on_the_three_caller_inputs() {
        assert!(free_roam_flush_context(true, true, true).can_show());
        assert!(!free_roam_flush_context(false, true, true).can_show());
        assert!(!free_roam_flush_context(true, false, true).can_show());
    }
}
