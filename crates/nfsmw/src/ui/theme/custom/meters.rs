//! The Custom meters: nitrous, boost and the speed breaker. Each is one row of the cluster (`drive_layout`): a small
//! label, then a slim `LinearProgress` bar that fills with its value. A meter the car does not have draws nothing, as the
//! Original gauge does not show without its system.
//!
//! The three slots share one component, [`Meter`], which takes the kind of meter it draws. State comes from the feed
//! (`drive_feed.rs`): `layout` reads it, `build` emits the row.

use blackbox_ui::{Component, DrawList, LayoutCx, LinearProgress, Rect, Text, justify, token_keys};

use super::drive_feed::{DriveFeed, DriveFeedCell};
use super::drive_layout;
use super::skin::drive;
use crate::i18n::t;

/// The boost that fills the boost bar, in psi. The bar is a scale of the usual boost of a road car, not a limit.
pub(super) const BOOST_FULL_PSI: f32 = 30.0;

/// The label's glyph scale.
const LABEL_SCALE: f32 = 1.0;

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

    /// The meter's label, in capitals as the cluster writes it.
    fn label(self) -> String {
        let text = match self {
            Self::Nitrous => t!("hud-meter-nitrous"),
            Self::Boost => t!("hud-meter-boost"),
            Self::Speedbreaker => t!("hud-meter-speedbreaker"),
        };
        text.to_uppercase()
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
        body.bar.build(out);
        body.label.build(out);
    }
}

/// The laid-out parts of a shown meter: the bar and the label.
struct Body {
    bar: LinearProgress,
    label: Text,
}

impl Body {
    fn of(kind: Kind, feed: &DriveFeed, cx: &LayoutCx<'_>) -> Option<Self> {
        let value = kind.value(feed)?;
        let row = kind.row();
        // The bar's corners come from the theme radius, and its border from the theme's border token.
        let mut bar = LinearProgress::new(drive::TRACK, kind.colour(), token_keys::BORDER).with_value(value);
        bar.layout(cx, drive_layout::meter_bar(feed.corner, row));
        let mut label = Text::new(drive::FONT, kind.label())
            .with_colour_key(drive::LABEL)
            .with_scale(LABEL_SCALE)
            .with_justification(justify::V_CENTRE);
        label.layout(cx, drive_layout::meter_label(feed.corner, row));
        Some(Self { bar, label })
    }
}
