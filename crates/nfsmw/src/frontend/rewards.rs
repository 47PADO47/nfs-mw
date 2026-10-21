//! The milestone rewards screen (`PostRace_MilestoneRewards.fng`): what an escape banked, a milestone and its bounty at a
//! time, and the bounty the car has now. The flow opens it with the chase's [`RewardsReport`] when the chase ended with
//! milestones banked (`post_race.rs`, `Frontend::poll_race`). Accept goes to the next milestone, then back to the game.
//! The package has one text line for the milestone and one for its bounty, so each milestone takes its turn.

use blackbox_feng::fe_hash_upper;
use blackbox_feng::ids::PAD_ACCEPT;

use super::ids;
use super::infractions::money;
use super::logic::{Command, Cx, ScreenLogic};
use crate::viewer::RewardsReport;

/// The milestone's line and its bounty line, and the total at the foot, by name hash.
const MILESTONE_LINE: u32 = 0xE526_D0D2;
const BOUNTY_LINE: u32 = 0xE104_5A4F;
const TOTAL_LINE: u32 = 0x324F_7792;

/// The big icon of the milestone, and its reveal script.
const ICON: u32 = 0xFCC8A;
const REVEAL: u32 = 0x5079_C8F8;

/// Shows a [`RewardsReport`].
pub struct RewardsLogic {
    report: RewardsReport,
    shown: usize,
}

impl RewardsLogic {
    pub fn new(report: RewardsReport) -> Self {
        Self { report, shown: 0 }
    }

    fn fill(&self, cx: &mut Cx) {
        let Some(line) = self.report.lines.get(self.shown) else { return };
        // The milestone's icon takes its texture and plays its reveal: the icon's script posts the messages that bring the
        // milestone line, the bounty line and the total in, one after the other.
        if let Some(icon) = cx.guid(ICON) {
            cx.rt.set_texture(icon, fe_hash_upper(line.icon));
            cx.rt.run_script(icon, REVEAL);
        }
        cx.text_object(MILESTONE_LINE, &line.text);
        cx.text_object(BOUNTY_LINE, &format!("+ {} Bounty", money(line.bounty)));
        cx.text_object(TOTAL_LINE, &format!("Total Bounty: {}", money(self.report.total_bounty)));
    }
}

impl ScreenLogic for RewardsLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.fill(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            // The package's own leave animation runs on accept; its end is the way out. With milestones left, accept shows
            // the next one instead of leaving.
            PAD_ACCEPT if self.shown + 1 < self.report.lines.len() => {
                self.shown += 1;
                self.fill(cx);
            }
            ids::EXIT_COMPLETE => cx.send(Command::Resume),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sample_report_lists_two_milestones() {
        let report = RewardsReport::sample();
        assert_eq!(report.lines.len(), 2);
        assert_eq!(report.lines.iter().map(|l| l.bounty).sum::<u32>(), 10_000);
    }
}
