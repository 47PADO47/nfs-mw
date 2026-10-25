//! The nitrous bar (`docs/specs/custom-hud/telemetry.md` section 6): [`SEGMENTS`] segments labelled `N2O`, hidden
//! entirely when the car has no nitrous.
//!
//! - **Charge:** the bar follows the charge through the same critically damped spring as the speed scale
//!   (`memory.rs`), so refills and burns glide.
//! - **Activation:** one pulse on the rising edge of `nos_burning`: the lit segments brighten to full and the label to
//!   `text`, fading back over `Motion::pulse` (400 ms) while the segments grow and settle. While the burn goes on the
//!   bar shows the depletion only: the pulse does not repeat until the burn has stopped. Under motion reduction the
//!   pulse is a static highlight held as long, with no growth.
//!
//! The bar is laid out with the others in `bars.rs`.

use super::super::tokens::{fade, keys};
use super::bars::{Bar, LIT_ALPHA};
use super::frame::Canvas;
use super::memory::Presented;
use crate::hud::HudState;
use crate::i18n::t;

/// The nitrous bar's segments.
pub const SEGMENTS: usize = 10;

/// The nitrous bar for `hud` with the presented charge and pulse, or `None` for a car without nitrous.
pub fn bar(canvas: &Canvas<'_>, hud: &HudState, presented: &Presented) -> Option<Bar> {
    if !hud.has_nos {
        return None;
    }
    let pulse = presented.nos_pulse.clamp(0.0, 1.0);
    let label_key = match pulse > 0.0 {
        true => keys::TEXT,
        false => keys::TEXT_SECONDARY,
    };
    let grow = match canvas.motion.pulse_moves {
        true => pulse,
        false => 0.0,
    };
    Some(Bar {
        label: t!("hud-cluster-nitrous"),
        label_key,
        segments: SEGMENTS,
        fill: presented.nos.clamp(0.0, 1.0),
        lit: fade(canvas.paint.colour(keys::TEXT), LIT_ALPHA + (1.0 - LIT_ALPHA) * pulse),
        grow,
    })
}
