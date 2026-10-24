//! The race card: the speed-trap points of a race, and the words of a result. The place, lap, clock, countdown and the
//! banner and race-over lines are the HUD package's (`hud/race.rs`); the points have no object there.

use egui::{Align2, Color32, RichText};

use crate::i18n::fmt::ordinal;
use crate::i18n::t;

/// What the race card keeps over the frame: the points of a speed-trap race (the HUD package has no object for them).
#[derive(Debug, Clone, PartialEq)]
pub struct RaceCard {
    /// Speed-trap points, when the race ranks by points.
    pub points: Option<f32>,
}

/// How the player's race ended.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultCard {
    pub place: u32,
    pub racers: usize,
    pub time: Option<f32>,
    /// A pursuit challenge: the heading says whether it was completed, not the place.
    pub challenge: bool,
}

impl ResultCard {
    /// The heading: the place, or that the race was lost without a finish.
    pub fn title(&self) -> String {
        if self.challenge {
            return match self.time {
                Some(_) => t!("hud-result-challenge-complete"),
                None => t!("hud-result-challenge-failed"),
            };
        }
        match (self.time, self.place) {
            (Some(_), 1) => t!("hud-result-won"),
            (Some(_), place) => t!("hud-result-placed", place = ordinal(place)),
            (None, _) => t!("hud-result-lost"),
        }
    }
}

/// Draws the card over the frame.
pub fn show(ctx: &egui::Context, card: &RaceCard) {
    let Some(points) = card.points else { return };
    egui::Area::new(egui::Id::new("race_status")).anchor(Align2::RIGHT_TOP, [-24.0, 96.0]).interactable(false).show(
        ctx,
        |ui| {
            egui::Frame::new()
                .fill(Color32::from_black_alpha(150))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    let text = t!("hud-race-points", points = format!("{points:.0}"));
                    ui.label(RichText::new(text).size(18.0).color(Color32::WHITE));
                });
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_follow_the_english_rules() {
        let text: Vec<String> = [1, 2, 3, 4, 11, 12, 13, 21, 22, 101, 111].into_iter().map(ordinal).collect();
        assert_eq!(text, ["1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd", "101st", "111th"]);
    }

    #[test]
    fn the_result_title_says_won_placed_or_lost() {
        let card = |place, time| ResultCard { place, racers: 4, time, challenge: false };
        assert_eq!(card(1, Some(50.0)).title(), "You won");
        assert_eq!(card(3, Some(50.0)).title(), "You finished 3rd");
        assert_eq!(card(4, None).title(), "Race lost");
        let challenge = |time| ResultCard { place: 1, racers: 2, time, challenge: true };
        assert_eq!(challenge(Some(90.0)).title(), "Challenge complete");
        assert_eq!(challenge(None).title(), "Challenge failed");
    }
}
