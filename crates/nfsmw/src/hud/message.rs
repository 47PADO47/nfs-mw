//! The message a mod shows (`bb.hud`): a centred line for a few seconds, drawn over the frame while a world is up. The
//! message is plain data and its timing ([`ModMessage`]); [`show`] is the thin egui view. The message is kept per thread,
//! as the mods' outbox is: the scenes and the mods run on the main thread.

use std::cell::RefCell;

use egui::{Align2, Color32, RichText};

/// How the message is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// A plain line.
    Info,
    /// A warning, in the warning colour.
    Warning,
    /// A large line.
    Big,
}

/// A message on screen, with the seconds it has left.
#[derive(Debug, Clone, PartialEq)]
pub struct ModMessage {
    pub text: String,
    pub tone: Tone,
    remaining: f32,
}

impl ModMessage {
    /// A message that stays for `secs` seconds (none if the time is not above zero).
    pub fn new(text: &str, tone: Tone, secs: f32) -> Self {
        Self { text: text.to_owned(), tone, remaining: secs.max(0.0) }
    }

    /// Count `dt` seconds off. Returns whether the message has run out.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.remaining -= dt.max(0.0);
        self.remaining <= 0.0
    }
}

thread_local! {
    static CARD: RefCell<Option<ModMessage>> = const { RefCell::new(None) };
}

/// Show `text` for `secs` seconds, in place of the message on screen.
pub fn set(text: &str, tone: Tone, secs: f32) {
    CARD.with(|card| *card.borrow_mut() = Some(ModMessage::new(text, tone, secs)));
}

/// Take the message down now.
pub fn hide() {
    CARD.with(|card| *card.borrow_mut() = None);
}

/// The message on screen, if any.
#[cfg(test)]
pub fn current() -> Option<ModMessage> {
    CARD.with(|card| card.borrow().clone())
}

/// Draw the message centred near the top of the window, and count its time off by the frame's time. The world's `ui`
/// calls it each frame.
pub fn show(ctx: &egui::Context) {
    let dt = ctx.input(|input| input.stable_dt);
    let drawn = CARD.with(|card| {
        let mut card = card.borrow_mut();
        let message = card.as_mut()?;
        if message.tick(dt) {
            *card = None;
            return None;
        }
        Some((message.text.clone(), message.tone))
    });
    let Some((text, tone)) = drawn else {
        return;
    };
    let (size, colour) = match tone {
        Tone::Info => (20.0, Color32::WHITE),
        Tone::Warning => (22.0, Color32::from_rgb(255, 196, 64)),
        Tone::Big => (40.0, Color32::WHITE),
    };
    egui::Area::new(egui::Id::new("mod_message")).anchor(Align2::CENTER_TOP, [0.0, 96.0]).interactable(false).show(
        ctx,
        |ui| {
            ui.label(RichText::new(text).size(size).strong().color(colour));
        },
    );
}
