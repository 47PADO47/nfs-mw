//! The end-of-pursuit card: the summary shown when a chase ends busted or evaded (the cops wrecked, the time). The card
//! is plain data and its fade by the seconds since the chase ended; [`show`] is the thin egui view. Spec:
//! `docs/specs/hud-pursuit.md` (end of pursuit).

use blackbox_pursuit::EndReason;
use egui::{Align2, Color32, RichText};

use super::pursuit::timer_text;
use crate::i18n::t;

/// Seconds the card stays up after the chase ended.
pub const SHOW_SECS: f32 = 6.0;
/// Seconds the card takes to fade in, and the same to fade out before [`SHOW_SECS`] runs out.
pub const FADE_SECS: f32 = 0.4;

/// What the card says: how the chase ended, the cops wrecked, the chase's length, and how long ago it ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PursuitEndCard {
    /// `Busted` or `Evaded`. A card is never shown for `None`.
    pub reason: EndReason,
    /// The cop cars wrecked in the chase.
    pub cops_destroyed: u32,
    /// The chase's length, seconds.
    pub secs: f32,
    /// Seconds since the chase ended.
    pub since_end: f32,
}

impl PursuitEndCard {
    /// Whether the card is on the screen: a chase that ended busted or evaded, less than [`SHOW_SECS`] ago.
    pub fn is_shown(&self) -> bool {
        self.reason != EndReason::None && self.since_end < SHOW_SECS
    }

    /// 0 hidden to 1 shown: the card fades in, holds, and fades out at the end.
    pub fn opacity(&self) -> f32 {
        let fade_in = self.since_end / FADE_SECS;
        let fade_out = (SHOW_SECS - self.since_end) / FADE_SECS;
        fade_in.min(fade_out).clamp(0.0, 1.0)
    }

    /// The card's heading.
    pub fn title(&self) -> String {
        match self.reason {
            EndReason::Evaded => t!("hud-pursuit-escaped"),
            EndReason::Busted => t!("hud-pursuit-busted"),
            EndReason::None => String::new(),
        }
    }
}

/// Draw the card over the frame, if one is up.
pub fn show(ctx: &egui::Context, card: &PursuitEndCard) {
    let opacity = card.opacity();
    if !card.is_shown() || opacity <= 0.0 {
        return;
    }
    let ink = |c: Color32| c.gamma_multiply(opacity);
    egui::Area::new(egui::Id::new("pursuit_end")).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).interactable(false).show(
        ctx,
        |ui| {
            egui::Frame::new()
                .fill(Color32::from_black_alpha(190).gamma_multiply(opacity))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    let title = RichText::new(card.title()).size(32.0).strong().color(ink(Color32::WHITE));
                    let wrecked = t!("hud-pursuit-cops-wrecked", n = card.cops_destroyed);
                    let time = t!("hud-pursuit-time", time = timer_text(card.secs));
                    ui.label(title);
                    ui.label(RichText::new(wrecked).size(18.0).color(ink(Color32::from_gray(210))));
                    ui.label(RichText::new(time).size(18.0).color(ink(Color32::from_gray(210))));
                });
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(reason: EndReason, since_end: f32) -> PursuitEndCard {
        PursuitEndCard { reason, cops_destroyed: 2, secs: 75.5, since_end }
    }

    #[test]
    fn only_a_busted_or_evaded_chase_has_a_card() {
        assert!(!card(EndReason::None, 0.0).is_shown());
        assert!(card(EndReason::Busted, 0.0).is_shown());
        assert!(card(EndReason::Evaded, 1.0).is_shown());
    }

    #[test]
    fn the_card_fades_in_holds_and_fades_out() {
        assert_eq!(card(EndReason::Evaded, 0.0).opacity(), 0.0);
        assert_eq!(card(EndReason::Evaded, 0.2).opacity(), 0.5);
        assert_eq!(card(EndReason::Evaded, 3.0).opacity(), 1.0);
        assert!((card(EndReason::Evaded, SHOW_SECS - 0.2).opacity() - 0.5).abs() < 1e-5);
        assert!(!card(EndReason::Evaded, SHOW_SECS).is_shown(), "gone at the end of the show time");
    }

    #[test]
    fn each_outcome_has_its_heading() {
        assert_eq!(card(EndReason::Evaded, 0.0).title(), "You escaped");
        assert_eq!(card(EndReason::Busted, 0.0).title(), "Busted");
    }
}
