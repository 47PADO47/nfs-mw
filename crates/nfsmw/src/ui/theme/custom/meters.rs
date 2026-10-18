//! The Custom meters: nitrous, boost and the speed breaker. Each is one row of the cluster (`drive_layout`): a small
//! label, then a thin bar that fills with its value. A meter the car does not have draws nothing, as the Original gauge
//! does not show without its system.
//!
//! The three slots share one component, [`Meter`], which takes the kind of meter it draws. State comes from the feed
//! (`drive_feed.rs`): `layout` reads it, `build` emits the row.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, Text, justify};

use super::drive_feed::{DriveFeed, DriveFeedCell};
use super::drive_layout;
use super::rounded::Rounded;
use super::skin::drive;

/// The boost that fills the boost bar, in psi. The bar is a scale of the usual boost of a road car, not a limit.
pub(super) const BOOST_FULL_PSI: f32 = 30.0;

/// The label's glyph scale.
const LABEL_SCALE: f32 = 1.0;
/// The meter bars' corner radius: the bars are 4 units tall, so this makes them pills.
const BAR_RADIUS: f32 = 2.0;

/// Which meter a [`Meter`] draws. Its row is the cluster row it sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Nitrous,
    Boost,
    Speedbreaker,
}

impl Kind {
    /// The row of the cluster this meter sits in: nitrous, boost, then the speed breaker.
    fn row(self) -> usize {
        match self {
            Self::Nitrous => 0,
            Self::Boost => 1,
            Self::Speedbreaker => 2,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Nitrous => "N2O",
            Self::Boost => "BOOST",
            Self::Speedbreaker => "SB",
        }
    }

    fn colour(self) -> &'static str {
        match self {
            Self::Nitrous => drive::NOS,
            Self::Boost => drive::BOOST,
            Self::Speedbreaker => drive::SPEEDBREAKER,
        }
    }

    /// The meter's value from the feed, 0 to 1, and whether the car shows the meter at all.
    fn value(self, feed: &DriveFeed) -> Option<f32> {
        match self {
            Self::Nitrous => feed.has_nos.then_some(feed.nos),
            Self::Boost => feed.has_turbo.then(|| (feed.boost_psi / BOOST_FULL_PSI).clamp(0.0, 1.0)),
            Self::Speedbreaker => (feed.speedbreaker > 0.0 || feed.speedbreaker_active).then_some(feed.speedbreaker),
        }
    }
}

/// The Custom meter component for one [`Kind`]. Built per resolve from the shared feed; it holds no state of its own.
pub struct Meter {
    kind: Kind,
    feed: DriveFeedCell,
    body: Option<Body>,
}

impl Meter {
    /// The meter of `kind`, reading `feed` (the cell the Custom theme holds).
    pub(super) fn new(kind: Kind, feed: DriveFeedCell) -> Self {
        Self { kind, feed, body: None }
    }
}

impl Component for Meter {
    /// The meter does not use `bounds`: it sits in its row of the cluster (`drive_layout`).
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.body = Body::of(self.kind, &self.feed.snapshot(), cx);
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        body.track.build(out);
        if let Some(fill) = &body.fill {
            fill.build(out);
        }
        body.label.build(out);
    }
}

/// The laid-out parts of a shown meter: the track, the fill (none at zero), and the label.
struct Body {
    track: Rounded,
    fill: Option<Rounded>,
    label: Text,
}

impl Body {
    fn of(kind: Kind, feed: &DriveFeed, cx: &LayoutCx<'_>) -> Option<Self> {
        let value = kind.value(feed)?;
        let row = kind.row();
        let bar = drive_layout::meter_bar(feed.corner, row);
        let track = Rounded::new(drive::TRACK, bar, BAR_RADIUS, cx);
        let fill = (value > 0.0).then(|| {
            let rect = drive_layout::at(bar.min_x(), bar.min_y(), bar.w * value, bar.h);
            Rounded::new(kind.colour(), rect, BAR_RADIUS, cx)
        });
        let mut label = Text::new(drive::FONT, kind.label())
            .with_colour_key(drive::LABEL)
            .with_scale(LABEL_SCALE)
            .with_justification(justify::V_CENTRE);
        label.layout(cx, drive_layout::meter_label(feed.corner, row));
        Some(Self { track, fill, label })
    }
}
