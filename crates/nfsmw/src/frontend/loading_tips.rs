//! The loading screen of the career (`Loading_Tips.fng`, docs/specs/loading-screen.md section 5): `LOADING...` at the
//! bottom left and a game tip with its picture in the middle. The package also has a `Press (key) To Continue` prompt for
//! the end of the load; the career does not use it, it goes straight into the world. The package only holds the
//! layout; this fills in the tip.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use blackbox_feng::resource_handle;
use blackbox_text::label_hash;

use super::logic::{Cx, ScreenLogic};
use crate::ui::tips::{self, TIPS};

/// The tip's text and title strings and its picture, by object name (`dump-screen Loading_Tips.fng`).
const BODY: u32 = 0xC5FB_C710;
const TITLE: u32 = 0x0D55_5245;
const PICTURE: u32 = 0xC9D7_7CB6;
/// The package's message that fades the tip group in.
const TIP_IN: u32 = 0xB237_48F4;
/// The tip the next career load shows: the first is picked by the clock, the ones after it follow in table order, so
/// loads in a row show different tips.
fn next_key() -> &'static str {
    static START: OnceLock<usize> = OnceLock::new();
    static LOADS: AtomicUsize = AtomicUsize::new(0);
    let start =
        *START.get_or_init(|| SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.subsec_nanos() as usize));
    TIPS[(start + LOADS.fetch_add(1, Ordering::Relaxed)) % TIPS.len()]
}

/// `Loading_Tips.fng` with the tip of its load.
pub struct LoadingTips {
    key: &'static str,
}

impl LoadingTips {
    pub fn new() -> Self {
        Self { key: next_key() }
    }

    #[cfg(test)]
    pub(super) fn with_key(key: &'static str) -> Self {
        Self { key }
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
    }

    fn message(&mut self, _cx: &mut Cx, _message: u32) {}
}

#[cfg(test)]
#[path = "loading_tips_tests.rs"]
mod tests;
