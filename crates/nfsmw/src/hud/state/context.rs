//! What the Custom HUD's contextual layouts read beyond the package's readout: where the player is, the stage of the race
//! and its intro, the entry card of the place the car stands in, the speed-trap points, the end of a chase and the line a
//! mod shows. The world fills it (`scenes/world/hud_context.rs`).
//!
//! The Original and Remastered HUDs draw the same things as egui cards from the world's own state
//! (`scenes/world/scene_hud.rs`); under the Custom HUD the cards are left out and the theme draws them from here.

use nfsmw_data::gameplay::RaceType;

use super::super::message::Tone;
use super::super::pursuit_end::PursuitEndCard;

/// Where a race stands, as the HUD lays itself out for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaceStage {
    /// From the grid loading to the countdown: the race's intro (its message, tutorial and scenes) or the wait for the
    /// start. The package's readout is not up yet.
    Intro,
    /// The countdown and the race.
    Running,
    /// The race-over message, the post-race screen and the outro: the results flow has the race.
    Results,
}

/// What the race intro card says. A field the race does not have is `None` or empty, never made up.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceIntro {
    /// The track's name in the language (`TRACKNAME_*`); `None` when the language has none.
    pub name: Option<String>,
    pub kind: RaceType,
    /// The laps of a lapped race (a circuit, a knockout); `None` for the others.
    pub laps: Option<u32>,
    /// The opponents' names, in grid order.
    pub opponents: Vec<String>,
}

/// What the Interact action does at the place the entry card offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryAction {
    /// Go into the place (a shop, a safe house, a race).
    GoIn,
    /// Leave the interior the player is in.
    Leave,
}

/// The entry card (`hud/interact_prompt.rs`): the place, its line, and how far the card has faded in.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryPrompt {
    pub title: String,
    /// The card's line as the world wrote it.
    pub line: String,
    /// What Interact does here; `None` for a line that is only information (a locked race, a placeholder, a mod's own
    /// wording).
    pub action: Option<EntryAction>,
    /// The card's fade, 0 hidden to 1 shown (`InteractPopup::opacity`).
    pub opacity: f32,
}

/// The context of the frame. Every field is absent where the world has nothing to say.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudContext {
    /// The player's car, render space (x, y), metres; `None` without a car.
    pub player: Option<[f32; 2]>,
    /// The stage of the race the player is in; `None` outside a race.
    pub race_stage: Option<RaceStage>,
    /// What the intro card of that race says; `None` outside a race.
    pub race_intro: Option<RaceIntro>,
    /// The points of a speed-trap race while it runs.
    pub race_points: Option<f32>,
    /// The entry card, while it is up.
    pub entry: Option<EntryPrompt>,
    /// The end of the last chase, while its card is up.
    pub pursuit_end: Option<PursuitEndCard>,
    /// The bounty the running chase has earned so far (its pending rep points), in career mode; `None` outside career
    /// mode and with no chase running.
    pub chase_bounty: Option<u32>,
    /// The line a mod shows (`bb.hud`), with its tone.
    pub mod_message: Option<(String, Tone)>,
}

/// What Interact does for an entry card with `line`: the world's two stock lines, `go_in` and `leave` (in the language
/// of the frame), name it. A card that only flashes (a placeholder, a locked race) and any other line have no action.
pub fn entry_action(line: &str, flashing: bool, go_in: &str, leave: &str) -> Option<EntryAction> {
    if flashing {
        return None;
    }
    if line == go_in {
        return Some(EntryAction::GoIn);
    }
    if line == leave {
        return Some(EntryAction::Leave);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stock_lines_name_the_action_and_a_flash_has_none() {
        let action = |line, flashing| entry_action(line, flashing, "Go in", "Leave");
        assert_eq!(action("Go in", false), Some(EntryAction::GoIn));
        assert_eq!(action("Leave", false), Some(EntryAction::Leave));
        assert_eq!(action("Go in", true), None, "a flashed card is not acted on");
        assert_eq!(action("Knock twice", false), None, "a mod's own wording is shown as it is");
    }

    #[test]
    fn an_empty_context_says_nothing() {
        let context = HudContext::default();
        assert_eq!((context.player, context.race_stage), (None, None));
        assert!(context.entry.is_none() && context.pursuit_end.is_none() && context.mod_message.is_none());
    }
}
