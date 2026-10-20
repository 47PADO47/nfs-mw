//! The race card: the banner, the speed-trap points and the result of a race. The place, lap, clock
//! and countdown are the HUD package's (`hud/race.rs`); this keeps what it has no object for.
//! [`RaceCard`] is plain data; [`show`] is the thin egui view.

use egui::{Align2, Color32, RichText};

/// The lines the race card keeps over the frame: the banner (the HUD package has no object for it), the points and
/// the result.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceCard {
    /// The banner (a checkpoint, a lap, a bonus) and its opacity, 0 to 1.
    pub banner: Option<(String, f32)>,
    /// Speed-trap points, when the race ranks by points.
    pub points: Option<f32>,
    /// The result, once the race is over.
    pub result: Option<ResultCard>,
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

/// `1st`, `2nd`, `3rd`, `4th` ... `11th`, `12th`, `21st`.
pub fn ordinal(n: u32) -> String {
    let suffix = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// A time as `m:ss.t`.
pub fn clock_text(secs: f32) -> String {
    let tenths = (secs.max(0.0) * 10.0).round() as u32;
    format!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
}

impl ResultCard {
    /// The heading: the place, or that the race was lost without a finish.
    pub fn title(&self) -> String {
        if self.challenge {
            return match self.time {
                Some(_) => "Challenge complete".to_owned(),
                None => "Challenge failed".to_owned(),
            };
        }
        match (self.time, self.place) {
            (Some(_), 1) => "You won".to_owned(),
            (Some(_), place) => format!("You finished {}", ordinal(place)),
            (None, _) => "Race lost".to_owned(),
        }
    }
}

/// Draws the card over the frame.
pub fn show(ctx: &egui::Context, card: &RaceCard) {
    if let Some(result) = &card.result {
        show_result(ctx, result);
    }
    if card.result.is_none() {
        show_status(ctx, card);
    }
    if let Some((text, opacity)) = &card.banner {
        let ink = Color32::WHITE.gamma_multiply(*opacity);
        egui::Area::new(egui::Id::new("race_banner"))
            .anchor(Align2::CENTER_CENTER, [0.0, -120.0])
            .interactable(false)
            .show(ctx, |ui| {
                ui.label(RichText::new(text).size(64.0).strong().color(ink));
            });
    }
}

fn show_status(ctx: &egui::Context, card: &RaceCard) {
    let Some(points) = card.points else { return };
    egui::Area::new(egui::Id::new("race_status")).anchor(Align2::RIGHT_TOP, [-24.0, 96.0]).interactable(false).show(
        ctx,
        |ui| {
            egui::Frame::new()
                .fill(Color32::from_black_alpha(150))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.label(RichText::new(format!("Speed {points:.0}")).size(18.0).color(Color32::WHITE));
                });
        },
    );
}

fn show_result(ctx: &egui::Context, result: &ResultCard) {
    egui::Area::new(egui::Id::new("race_result")).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).interactable(false).show(
        ctx,
        |ui| {
            egui::Frame::new()
                .fill(Color32::from_black_alpha(200))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.label(RichText::new(result.title()).size(34.0).strong().color(Color32::WHITE));
                    ui.label(
                        RichText::new(format!("Place {} of {}", result.place, result.racers))
                            .size(20.0)
                            .color(Color32::from_gray(215)),
                    );
                    if let Some(time) = result.time {
                        ui.label(
                            RichText::new(format!("Time {}", clock_text(time)))
                                .size(20.0)
                                .color(Color32::from_gray(215)),
                        );
                    }
                    ui.label(RichText::new("Press Enter to continue").size(14.0).color(Color32::from_gray(160)));
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
    fn the_clock_reads_minutes_seconds_and_tenths() {
        assert_eq!(clock_text(0.0), "0:00.0");
        assert_eq!(clock_text(65.43), "1:05.4");
        assert_eq!(clock_text(600.0), "10:00.0");
        assert_eq!(clock_text(-3.0), "0:00.0");
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
