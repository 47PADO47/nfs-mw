//! The free-roam extras (`[custom_hud] free_roam_extras`, on in the Advanced preset): the career's cash, the bounty and
//! the distance along the GPS route, as one line under the radio card's zone. Each value shows only when the game has
//! it: outside a career there is no cash or bounty, and without a route no distance. Drawn in the `BountyRow` slot,
//! whose Original row belongs to the pursuit board; in a chase the bounty is the pursuit panel's.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

use super::super::feed::{CustomFeed, CustomFeedCell};
use super::super::layout::{Anchor, HudBox};
use super::super::memory::Frame;
use super::super::mode::HudMode;
use super::super::parts::{Drawn, Parts, left};
use super::super::text::{Label, size};
use super::super::tokens::{Motion, keys};
use super::super::words::{distance_long, grouped};
use super::{BELOW_RADIO, EXTRAS_HEIGHT};
use crate::hud::HudState;
use crate::i18n::t;
use crate::settings::SpeedUnits;

/// The line's width at scale 1.
const WIDTH: f32 = 260.0;

/// The values of the line, each present only when the game has it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtrasLine {
    pub cash: Option<u32>,
    pub bounty: Option<u32>,
    /// Metres along the GPS route to its end.
    pub route_m: Option<f32>,
}

impl ExtrasLine {
    /// The line of `hud`.
    pub fn of(hud: &HudState) -> Self {
        let route_m = hud.route.as_ref().zip(hud.context.player).and_then(|(route, at)| along_route(&route.path, at));
        Self { cash: hud.cash, bounty: hud.career_bounty, route_m }
    }

    /// The line's text, its values apart by wide spaces; `None` when there is no value.
    pub fn text(&self, units: SpeedUnits) -> Option<String> {
        let values: Vec<String> = [
            self.cash.map(|cash| t!("hud-extras-cash", cash = grouped(cash))),
            self.bounty.map(|bounty| t!("hud-extras-bounty", bounty = grouped(bounty))),
            self.route_m.map(|metres| t!("hud-extras-route", distance = distance_long(metres, units))),
        ]
        .into_iter()
        .flatten()
        .collect();
        if values.is_empty() {
            return None;
        }
        Some(values.join("   "))
    }
}

/// Metres along `path` from the point of it nearest `at` to its end, plus the way from `at` to that point. `None` for
/// an empty path.
pub fn along_route(path: &[[f32; 2]], at: [f32; 2]) -> Option<f32> {
    let length = |a: [f32; 2], b: [f32; 2]| (b[0] - a[0]).hypot(b[1] - a[1]);
    let nearest = (0..path.len()).min_by(|&a, &b| length(path[a], at).total_cmp(&length(path[b], at)))?;
    let rest: f32 = path[nearest..].windows(2).map(|pair| length(pair[0], pair[1])).sum();
    Some(length(at, path[nearest]) + rest)
}

/// The Custom `BountyRow` component: the extras line in free roam.
pub struct Extras {
    feed: CustomFeedCell,
    drawn: Drawn,
}

impl Extras {
    pub fn new(feed: CustomFeedCell) -> Self {
        Self { feed, drawn: Drawn::default() }
    }
}

impl Component for Extras {
    /// The line places itself in the HUD's frame, not in `bounds`.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.get();
        let frame = self.feed.memory().frame(&feed);
        self.drawn = draw(&feed, &frame, cx);
    }

    fn build(&self, out: &mut DrawList) {
        self.drawn.build(out);
    }
}

/// The line of the frame: in free roam, with the option on and a value to show.
fn draw(feed: &CustomFeed, frame: &Frame, cx: &LayoutCx<'_>) -> Drawn {
    if !feed.settings.free_roam_extras {
        return Drawn::default();
    }
    let fade = frame.opacity(|mode| mode == HudMode::FreeRoam, &Motion::for_settings(&feed.settings));
    if fade <= 0.0 {
        return Drawn::default();
    }
    let Some(text) = ExtrasLine::of(&feed.hud).text(feed.units) else { return Drawn::default() };
    let mut parts = Parts::new(cx, HudBox::new(feed.screen, feed.settings.scale), &feed.settings);
    parts.fade(fade);
    let rect = parts.place(Anchor::TopLeft, (0.0, BELOW_RADIO), (WIDTH, EXTRAS_HEIGHT));
    let label = Label::label(text, size::NOTIFICATION_LINE).colour(keys::TEXT_SECONDARY).justify(left());
    parts.text(label, rect);
    parts.finish()
}
