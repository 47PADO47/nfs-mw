//! The Custom drive cluster: the horizontal telemetry instrument in the bottom-right corner of the content box
//! (`docs/specs/custom-hud/telemetry.md`, on the PR 1 branch; what was built is in `docs/specs/custom-hud-cluster.md`).
//! Written for the Custom HUD 2026; the Remastered cluster's corner was the inspiration, none of its code.
//!
//! Top to bottom: the advanced row (advanced telemetry only), the rev strip, the speed row on its plate (the gear box,
//! the speed digits and the unit), the segmented speed scale, and the bar row (nitrous, speedbreaker, boost). One
//! component, [`Cluster`], draws all of it from the `Speedometer` slot; the theme claims the tachometer, gear, nitrous,
//! turbo and speedbreaker slots with nothing, so the Original pieces do not draw under it.
//!
//! The cluster reads [`CustomFeedCell`] every frame and keeps what animates between frames (the springs and pulses) in
//! the [`ClusterMemory`] the cell carries, since the theme builds a new component every frame. It honours
//! `[custom_hud]` scale, opacity, text size, high contrast and the palette through the layout and the tokens.
//!
//! Submodules:
//!
//! - [`frame`]: the rows, their place on screen, the colours of a frame, and the [`frame::Canvas`] the parts draw on.
//! - [`memory`]: the springs, the pulses and the speedbreaker's rest, kept between frames.
//! - [`speed`]: the plate, the digits and the unit.
//! - [`scale`]: the smoothed speed scale.
//! - [`revs`]: the rev strip, the gear box and the shift cue.
//! - [`nitrous`]: the nitrous bar and its activation pulse.
//! - [`bars`]: the bar row, with the speedbreaker and boost.
//! - [`advanced`]: the advanced row.

mod advanced;
mod bars;
mod frame;
mod memory;
mod nitrous;
mod revs;
mod scale;
mod speed;

pub use memory::ClusterMemory;

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

use super::feed::{CustomFeed, CustomFeedCell};
use frame::{Canvas, Parts};
use speed::SpeedRow;

/// The Custom cluster component. Built per resolve from the shared feed; its memory lives in the feed cell.
pub struct Cluster {
    feed: CustomFeedCell,
    parts: Parts,
}

impl Cluster {
    /// The cluster that reads `feed` (the cell the Custom theme holds).
    pub fn new(feed: CustomFeedCell) -> Self {
        Self { feed, parts: Vec::new() }
    }
}

impl Component for Cluster {
    /// The cluster does not use `bounds`: it places itself in the content box from the feed's window.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts = parts(&self.feed.get(), self.feed.cluster(), *cx);
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// The cluster's parts for one frame of `feed`, back to front, after stepping `memory` to the frame.
fn parts(feed: &CustomFeed, memory: &ClusterMemory, cx: LayoutCx<'_>) -> Parts {
    let hud = &feed.hud;
    let cue = revs::shift_cue(hud, feed.transmission);
    let presented = memory.step(feed, cue);
    let mut canvas = Canvas::new(feed, cx);
    let row = SpeedRow::of(&canvas.frame);
    speed::plate(&mut canvas, &row);
    revs::gear(&mut canvas, hud, cue, presented.cue_pulse, &row);
    speed::readout(&mut canvas, hud, feed.units, &row);
    scale::draw(&mut canvas, presented.speed, feed.units);
    revs::strip(&mut canvas, hud);
    let shown: Vec<bars::Bar> = [
        nitrous::bar(&canvas, hud, &presented),
        bars::speedbreaker(&canvas, hud, &presented),
        bars::boost(&canvas, hud),
    ]
    .into_iter()
    .flatten()
    .collect();
    bars::draw(&mut canvas, &shown);
    if feed.settings.advanced_telemetry {
        advanced::draw(&mut canvas, hud);
    }
    canvas.parts
}
