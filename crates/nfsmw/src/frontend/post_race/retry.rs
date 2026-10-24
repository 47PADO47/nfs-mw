//! The question a career race that was not won asks when its results are left (`docs/specs/race-loss-retry.md`). A race the
//! story chains on from can only be run again; any other career race may also be given up.

use crate::frontend::confirm::{Alert, Confirm, Look};
use crate::frontend::logic::Command;
use crate::ui::Text;
use crate::viewer::RaceAnswer;

/// A career race with no race chained after it: Retry runs it again, Quit leaves it for free roam. Retry has the focus, and
/// Back closes the question and leaves the results where they were.
pub fn offer() -> Confirm {
    Confirm::choice(
        Text::Literal("You did not win this race. Try it again?".into()),
        (Text::Literal("Retry".into()), Command::RaceAnswer(RaceAnswer::Restart)),
        (Text::Literal("Quit".into()), Command::RaceAnswer(RaceAnswer::Exit)),
    )
    .looking(Look::Attention)
    .focusing_first(true)
}

/// A race the story goes on from: closing the notice (its one answer, or Back) runs the race again.
pub fn required() -> Alert {
    Alert {
        title: Text::Literal("Race lost".into()),
        message: Text::Literal("You have to win this race to go on. It starts again.".into()),
        on_close: Some(Box::new(Command::RaceAnswer(RaceAnswer::Restart))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offer_retries_first_and_quits_second() {
        let offer = offer();
        assert_eq!(*offer.on_yes, Command::RaceAnswer(RaceAnswer::Restart));
        assert_eq!(offer.on_no.as_deref(), Some(&Command::RaceAnswer(RaceAnswer::Exit)));
        assert!(offer.focus_yes, "Retry has the focus");
    }

    #[test]
    fn the_required_retry_has_no_way_out() {
        let notice = required();
        assert_eq!(notice.on_close.as_deref(), Some(&Command::RaceAnswer(RaceAnswer::Restart)));
    }
}
