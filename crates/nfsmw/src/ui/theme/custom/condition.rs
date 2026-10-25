//! The vehicle condition chip (`docs/specs/custom-hud/modes.md` section 4), slot `VehicleCondition`, bottom-left: a
//! small chip that expands on a real warning. It is not a health bar.
//!
//! - **Hidden** while the car is sound, and for a car the simulation gives no condition.
//! - **Compact**: a car icon and `Damaged`, once the body passed [`LIGHT_DAMAGE`] or a tyre is down.
//! - **Expanded** on a warning (`VehicleCondition::warning`: a tyre punctured or blown, heavy body damage, the car
//!   disabled): the icon, the worst warning's label and, for a tyre, the wheel. It stays expanded while the warning
//!   holds and [`COLLAPSE_SECS`] after it clears.
//!
//! The icon and the label carry the state; the caution and critical colours only reinforce it. The disabled warning is a
//! safety message, drawn at no less than `SAFETY_OPACITY`. The memory of the last warning ([`ConditionMemory`]) lives in
//! the layouts' memory cell (`memory.rs`), since the component is rebuilt every frame.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};
use glam::Vec2;

use super::feed::{CustomFeed, CustomFeedCell};
use super::layout::{Anchor, HudBox};
use super::memory::Frame;
use super::mode::HudMode;
use super::parts::{Drawn, Parts, left};
use super::plate::PlateStyle;
use super::text::{Label, size};
use super::tokens::{SAFETY_OPACITY, keys};
use crate::hud::{ConditionWarning, TireCondition, VehicleCondition};
use crate::i18n::t;

/// The body damage from which the chip shows compact: a presentation threshold, not a game value.
pub const LIGHT_DAMAGE: f32 = 0.15;
/// How long the chip stays expanded after its warning clears, seconds.
pub const COLLAPSE_SECS: f32 = 4.0;

/// What the chip shows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ChipState {
    #[default]
    Hidden,
    Compact,
    /// The warning, and for a tyre the wheel (front left, front right, rear left, rear right).
    Expanded {
        warning: ConditionWarning,
        wheel: Option<usize>,
    },
}

/// The last warning and when it last held, so the chip collapses only [`COLLAPSE_SECS`] after it cleared.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ConditionMemory {
    last: Option<(ConditionWarning, Option<usize>, f32)>,
}

impl ConditionMemory {
    /// Takes the frame's condition at `now` and says what the chip shows.
    pub fn update(&mut self, condition: Option<&VehicleCondition>, now: f32) -> ChipState {
        let Some(condition) = condition else {
            self.last = None;
            return ChipState::Hidden;
        };
        if let Some(warning) = condition.warning() {
            let wheel = wheel_of(condition, warning);
            self.last = Some((warning, wheel, now));
            return ChipState::Expanded { warning, wheel };
        }
        if let Some((warning, wheel, at)) = self.last
            && now - at < COLLAPSE_SECS
        {
            return ChipState::Expanded { warning, wheel };
        }
        self.last = None;
        match reports(condition) {
            true => ChipState::Compact,
            false => ChipState::Hidden,
        }
    }
}

/// Whether a condition has anything to report, short of a warning: body damage past [`LIGHT_DAMAGE`], or a tyre down.
fn reports(condition: &VehicleCondition) -> bool {
    condition.damaged_tires() > 0 || condition.cosmetic.is_some_and(|level| level >= LIGHT_DAMAGE)
}

/// The wheel a tyre warning is about: the first wheel in that state.
fn wheel_of(condition: &VehicleCondition, warning: ConditionWarning) -> Option<usize> {
    let state = match warning {
        ConditionWarning::TireBlown => TireCondition::Blown,
        ConditionWarning::TirePunctured => TireCondition::Punctured,
        _ => return None,
    };
    condition.tires.iter().position(|tire| *tire == state)
}

/// The chip's label for a warning, with the wheel for a tyre.
pub fn warning_label(warning: ConditionWarning, wheel: Option<usize>) -> String {
    let label = match warning {
        ConditionWarning::Critical => t!("hud-condition-disabled"),
        ConditionWarning::TireBlown => t!("hud-condition-tire-blown"),
        ConditionWarning::TirePunctured => t!("hud-condition-tire-punctured"),
        ConditionWarning::HeavyDamage => t!("hud-condition-heavy-damage"),
    };
    let Some(wheel) = wheel.and_then(wheel_name) else { return label };
    format!("{label} \u{b7} {wheel}")
}

/// A wheel's name, in wheel order.
fn wheel_name(wheel: usize) -> Option<String> {
    match wheel {
        0 => Some(t!("hud-wheel-front-left")),
        1 => Some(t!("hud-wheel-front-right")),
        2 => Some(t!("hud-wheel-rear-left")),
        3 => Some(t!("hud-wheel-rear-right")),
        _ => None,
    }
}

/// The chip's size at scale 1, compact and expanded, and its icon's side.
const COMPACT: (f32, f32) = (78.0, 20.0);
const EXPANDED: (f32, f32) = (190.0, 22.0);
const ICON: f32 = 14.0;

/// The Custom `VehicleCondition` component. Built per resolve from the shared feed; its memory is the feed cell's.
pub struct ConditionChip {
    feed: CustomFeedCell,
    drawn: Drawn,
}

impl ConditionChip {
    pub fn new(feed: CustomFeedCell) -> Self {
        Self { feed, drawn: Drawn::default() }
    }
}

impl Component for ConditionChip {
    /// The chip places itself in the HUD's frame, not in `bounds`.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.get();
        let frame = self.feed.memory().frame(&feed);
        self.drawn = draw(&feed, &frame, cx);
    }

    fn build(&self, out: &mut DrawList) {
        self.drawn.build(out);
    }
}

/// The chip of the frame.
fn draw(feed: &CustomFeed, frame: &Frame, cx: &LayoutCx<'_>) -> Drawn {
    let fade = frame.opacity(|mode| mode != HudMode::Hidden, &super::tokens::Motion::for_settings(&feed.settings));
    if fade <= 0.0 {
        return Drawn::default();
    }
    let mut parts = Parts::new(cx, HudBox::new(feed.screen, feed.settings.scale), &feed.settings);
    parts.fade(fade);
    match frame.condition {
        ChipState::Hidden => {}
        ChipState::Compact => {
            let rect = parts.place(Anchor::BottomLeft, (0.0, 0.0), COMPACT);
            parts.plate(rect, PlateStyle::SURFACE);
            chip_row(
                &mut parts,
                rect,
                keys::CAUTION,
                Label::heading(t!("hud-condition-damaged").to_uppercase(), size::HEADING),
            );
        }
        ChipState::Expanded { warning, wheel } => {
            let critical = warning == ConditionWarning::Critical;
            if critical {
                parts.at_least(SAFETY_OPACITY);
            }
            let rect = parts.place(Anchor::BottomLeft, (0.0, 0.0), EXPANDED);
            let (fill, icon, text) = match critical {
                true => (keys::CRITICAL, keys::CRITICAL_TEXT, keys::CRITICAL_TEXT),
                false => (keys::SURFACE_RAISED, keys::CAUTION, keys::TEXT),
            };
            parts.plate(rect, PlateStyle { fill, ..PlateStyle::RAISED }.with_accent(icon));
            let label = Label::label(warning_label(warning, wheel), size::NOTIFICATION_LINE).colour(text);
            chip_row(&mut parts, rect, icon, label);
        }
    }
    parts.finish()
}

/// The icon at the left of `rect` in `icon_key`, then `label` beside it.
fn chip_row(parts: &mut Parts<'_>, rect: Rect, icon_key: &'static str, label: Label) {
    let s = parts.scale();
    let pad = 4.0 * s;
    let side = ICON * s;
    let icon = Rect::new(rect.min_x() + pad + side / 2.0, rect.y, side, side);
    parts.stroke(car_outline(icon), 1.2, icon_key);
    let text_left = icon.max_x() + pad;
    let text = super::parts::edges(text_left, rect.min_y(), rect.max_x() - pad, rect.max_y());
    parts.text(label.justify(left()), text);
}

/// A car seen from the side, as a closed outline in `rect`: the body, then the cabin.
pub fn car_outline(rect: Rect) -> Vec<Vec2> {
    let at = |u: f32, v: f32| Vec2::new(rect.min_x() + u * rect.w, rect.min_y() + v * rect.h);
    vec![
        at(0.05, 0.85),
        at(0.05, 0.55),
        at(0.25, 0.5),
        at(0.38, 0.25),
        at(0.68, 0.25),
        at(0.8, 0.5),
        at(0.95, 0.55),
        at(0.95, 0.85),
        at(0.05, 0.85),
    ]
}

#[cfg(test)]
#[path = "condition_tests.rs"]
mod tests;
