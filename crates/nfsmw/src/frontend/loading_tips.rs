//! The loading screen of the career (`Loading_Tips.fng`, docs/specs/loading-screen.md section 5): `LOADING...` at the
//! bottom left and a game tip with its picture in the middle. The package also has a `Press (key) To Continue` prompt for
//! the end of the first safe-house arrival. Other tips advance automatically.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use blackbox_feng::ids::{BUTTON_PRESSED, PAD_ACCEPT};
use blackbox_feng::resource_handle;
use blackbox_text::label_hash;

use super::logic::{Command, Cx, ScreenLogic};
use crate::ui::tips::{self, TIPS};

/// The tip's text and title strings and its picture, by object name (`dump-screen Loading_Tips.fng`).
const BODY: u32 = 0xC5FB_C710;
const TITLE: u32 = 0x0D55_5245;
const PICTURE: u32 = 0xC9D7_7CB6;
/// The package's message that fades the tip group in.
const TIP_IN: u32 = 0xB237_48F4;
const SHOW_CONTINUE: u32 = 0x9938_A38F;
const INTRO_TIP_SECONDS: f32 = 5.0;
/// The tip the next career load shows: the first is picked by the clock, the ones after it follow in table order, so
/// loads in a row show different tips.
fn next_key() -> &'static str {
    static START: OnceLock<usize> = OnceLock::new();
    static LOADS: AtomicUsize = AtomicUsize::new(0);
    let start = *START.get_or_init(first_tip);
    TIPS[(start + LOADS.fetch_add(1, Ordering::Relaxed)) % TIPS.len()]
}

/// Where the tips start: the run's seed when it has one (`--seed`, `parity/seed.rs`), else the clock.
fn first_tip() -> usize {
    if let Some(seed) = crate::parity::seed::fixed(crate::parity::seed::Stream::LoadingTips) {
        return (seed % TIPS.len() as u64) as usize;
    }
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.subsec_nanos() as usize)
}

/// `Loading_Tips.fng` with the tip of its load.
pub struct LoadingTips {
    key: &'static str,
    intro_left: Option<f32>,
}

impl LoadingTips {
    pub fn new() -> Self {
        Self { key: next_key(), intro_left: None }
    }

    pub(super) fn first_safe_house() -> Self {
        Self { key: "ENGAGE_MECHANIC", intro_left: Some(INTRO_TIP_SECONDS) }
    }

    #[cfg(test)]
    pub(super) fn with_key(key: &'static str) -> Self {
        Self { key, intro_left: None }
    }
}

impl ScreenLogic for LoadingTips {
    fn start(&mut self, cx: &mut Cx) {
        if let Some(title) = cx.object(TITLE) {
            cx.label(title, label_hash(&tips::header_label(self.key)));
        }
        if let Some(body) = cx.object(BODY) {
            cx.label(body, label_hash(&tips::description_label(self.key)));
        }
        if let Some(picture) = cx.object(PICTURE) {
            cx.rt.set_texture(picture, resource_handle(&tips::picture_name(self.key)));
        }
        cx.rt.post_to_package(cx.package, TIP_IN);
        if self.intro_left.is_some() {
            cx.rt.set_input_enabled(cx.package, false);
        }
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        let Some(left) = self.intro_left.as_mut().filter(|left| **left > 0.0) else { return };
        *left -= dt;
        if *left <= 0.0 {
            cx.rt.post_to_package(cx.package, SHOW_CONTINUE);
        }
    }

    fn accepts_input(&self) -> bool {
        self.intro_left.is_none_or(|left| left <= 0.0)
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if matches!(message, PAD_ACCEPT | BUTTON_PRESSED) && self.intro_left.is_some_and(|left| left <= 0.0) {
            self.intro_left = None;
            cx.send(Command::ContinueFirstSafeHouse);
        }
    }
}

#[cfg(test)]
#[path = "loading_tips_tests.rs"]
mod tests;
