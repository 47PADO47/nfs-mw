//! The interact prompt: the small card at the top left that offers an entry when the car drives into a place (an auto
//! shop, a safe house, a race start), animated in and out. [`InteractPopup`] is plain data and the timing; [`show`] is
//! the thin egui view. Spec: `docs/specs/world-markers.md` section 4.

use egui::{Color32, RichText, pos2};

/// Seconds to fade in, or out before another prompt takes its place.
pub(super) const FADE_SECS: f32 = 0.2;
/// How far the card slides in from the left, points.
pub(super) const SLIDE: f32 = 24.0;
/// The card's place from the window's top left, points.
const MARGIN: f32 = 24.0;

/// What the card says: the place, and the line under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub title: String,
    pub line: String,
}

/// The card's state: which prompt it shows, how far it has faded in, and for how long a flashed prompt holds the card.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InteractPopup {
    /// The prompt the card should show now (`None` hides it).
    target: Option<Prompt>,
    /// The prompt on the card, which trails `target` while the card fades out.
    drawn: Option<Prompt>,
    /// 0 hidden to 1 fully shown.
    alpha: f32,
    /// Seconds left of a flashed prompt; while it runs, `set_target` does not change the card.
    hold: f32,
}

impl InteractPopup {
    /// The prompt the car's position asks for (`None` when there is none). Ignored while a flashed prompt holds.
    pub fn set_target(&mut self, prompt: Option<Prompt>) {
        if self.hold > 0.0 {
            return;
        }
        self.target = prompt;
    }

    /// Show `prompt` for `secs` seconds whatever the car is doing (the placeholder after an enter).
    pub fn flash(&mut self, prompt: Prompt, secs: f32) {
        self.target = Some(prompt);
        self.hold = secs.max(0.0);
    }

    /// Advance the fade by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        let mut dt = dt.max(0.0);
        self.hold = (self.hold - dt).max(0.0);
        if self.drawn != self.target {
            // Fade the old card out; the new one takes over at 0 and fades in with the time left over.
            let out_secs = self.alpha * FADE_SECS;
            if dt < out_secs {
                self.alpha -= dt / FADE_SECS;
                return;
            }
            dt -= out_secs;
            self.alpha = 0.0;
            self.drawn = self.target.clone();
        }
        if self.drawn.is_none() {
            self.alpha = 0.0;
            return;
        }
        self.alpha = (self.alpha + dt / FADE_SECS).min(1.0);
    }

    /// A flashed prompt holds the card (see [`Self::flash`]).
    pub fn is_flashing(&self) -> bool {
        self.hold > 0.0
    }

    /// Whether the card draws anything: a prompt is on it and it is not faded out.
    pub fn visible(&self) -> bool {
        self.drawn.is_some() && self.alpha > 0.0
    }

    /// The prompt on the card, if any.
    pub fn drawn(&self) -> Option<&Prompt> {
        self.drawn.as_ref()
    }

    /// The card is up and nearly fully shown: the player can act on the prompt (an enter is not taken mid-fade).
    pub fn is_up(&self) -> bool {
        self.target.is_some() && self.drawn == self.target && self.alpha >= 0.5
    }

    /// 0 hidden to 1 shown.
    pub fn opacity(&self) -> f32 {
        self.alpha
    }

    /// How far the card is still out to the left, points: `SLIDE` when hidden, 0 when shown.
    pub fn slide(&self) -> f32 {
        -SLIDE * (1.0 - self.alpha)
    }
}

/// Draw the card, if one is up.
pub fn show(ctx: &egui::Context, popup: &InteractPopup) {
    if !popup.visible() {
        return;
    }
    let Some(prompt) = popup.drawn() else { return };
    let opacity = popup.opacity();
    let ink = |c: Color32| c.gamma_multiply(opacity);
    let at = pos2(MARGIN + popup.slide(), MARGIN);
    egui::Area::new(egui::Id::new("interact_prompt")).fixed_pos(at).interactable(false).show(ctx, |ui| {
        egui::Frame::new()
            .fill(Color32::from_black_alpha(160).gamma_multiply(opacity))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                let title = RichText::new(&prompt.title).size(16.0).strong().color(ink(Color32::WHITE));
                let line = RichText::new(&prompt.line).size(13.0).color(ink(Color32::from_gray(210)));
                ui.add(egui::Label::new(title).wrap_mode(egui::TextWrapMode::Extend));
                ui.add(egui::Label::new(line).wrap_mode(egui::TextWrapMode::Extend));
            });
    });
}
