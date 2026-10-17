//! The first visit: stage 16 becomes 15 (`docs/specs/career-safehouse.md` section 5.3, last paragraph).
//!
//! `DoSafeHouseIntro` sets stage 16 and opens the movie `storyfmv_saf25`, then the rival bio screen. The bio
//! screen constructor, seeing the Blacklist sequence bit and stage 16, posts the enter-bin event with bin 15: the
//! stage byte walks down from 16 to 15. That is the only native place that turns 16 into 15.
//!
//! What the enter-bin routine also does (update the gameplay manager for each bin passed, activate the bin event
//! nodes, grant the bin rewards into the popup queue) belongs to the progress slice and is not modelled. The spec
//! lists no SMS for this step: bin 15 has no entry message (`career-scripts.md` 3.3) and message 15 was queued by the
//! prologue drive, so the phone queue is not touched here.

use super::context::FrontEndContext;
use crate::settings::{CareerSettings, FIRST_RIVAL_STAGE, PROLOGUE_STAGE};

/// The movie the first visit plays before the bio screen.
pub const FIRST_VISIT_MOVIE: &str = "storyfmv_saf25";

/// What entering the first Blacklist bin did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstVisit {
    /// The bin that was entered (15).
    pub entered_bin: u8,
}

/// The rival bio screen opens: with the Blacklist sequence bit set and stage 16, the stage becomes 15. Any other
/// combination changes nothing (an ordinary visit after a rival is beaten is at a lower stage already).
pub fn bio_screen_opened(settings: &mut CareerSettings, context: FrontEndContext) -> Option<FirstVisit> {
    if !context.contains(FrontEndContext::BLACKLIST_SEQUENCE) || settings.stage != PROLOGUE_STAGE {
        return None;
    }
    settings.stage = FIRST_RIVAL_STAGE;
    Some(FirstVisit { entered_bin: FIRST_RIVAL_STAGE })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_16_becomes_15_in_the_sequence() {
        let mut s = CareerSettings::new_career();
        let before = s.clone();
        let visit = bio_screen_opened(&mut s, FrontEndContext::CAREER.with_sequence());
        assert_eq!(visit, Some(FirstVisit { entered_bin: 15 }));
        assert_eq!(s.stage, 15);
        assert_eq!(s.cash, before.cash);
        assert_eq!(s.flags, before.flags, "no flag changes at this step");
        assert_eq!(s.sms, before.sms, "no SMS at this step");
    }

    #[test]
    fn it_happens_once() {
        let mut s = CareerSettings::new_career();
        assert!(bio_screen_opened(&mut s, FrontEndContext::CAREER.with_sequence()).is_some());
        assert_eq!(bio_screen_opened(&mut s, FrontEndContext::CAREER.with_sequence()), None);
        assert_eq!(s.stage, 15);
    }

    #[test]
    fn without_the_sequence_bit_or_at_another_stage_nothing_changes() {
        let mut s = CareerSettings::new_career();
        assert_eq!(bio_screen_opened(&mut s, FrontEndContext::CAREER), None);
        assert_eq!(s.stage, 16);
        s.stage = 12;
        assert_eq!(bio_screen_opened(&mut s, FrontEndContext::CAREER.with_sequence()), None);
        assert_eq!(s.stage, 12);
    }

    impl FrontEndContext {
        fn with_sequence(self) -> FrontEndContext {
            FrontEndContext(self.0 | FrontEndContext::BLACKLIST_SEQUENCE.0)
        }
    }
}
