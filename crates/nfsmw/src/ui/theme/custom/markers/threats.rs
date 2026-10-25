//! The police threat indicators (`docs/specs/custom-hud/world-markers.md` section 3, `fair-knowledge.md`): where the
//! confirmed threats are, as edge arrows while they are outside the view or small marks inside it, and where the police
//! last saw the player, in a hollow style that fades with its age.
//!
//! Fair knowledge is kept by reading only `HudState::police_intel`, which holds a cop's place only while that cop has
//! the player in sight, and the helicopter's only while it does. The last-known place is not a cop's place: it is shown
//! only while no threat is confirmed, hollow, never as a cop. The indicators show while a chase runs
//! (`pursuit_phase` not `None`) with `[custom_hud] pursuit_info` on.
//!
//! The intel is across the ground (x, y). The marks are given the car's height (`WorldMarks::car_z`), and the
//! helicopter [`HELICOPTER_LIFT`] above it, which is a drawing height, not the helicopter's altitude.

use blackbox_ui::Rect;
use glam::Vec2;

use crate::hud::{HudCamera, HudState, PursuitPhase};
use crate::settings::hud::HudSettings;

use super::project::{self, Placement};
use super::rules::across;

/// The most police indicators at once, nearest first.
pub const MAX_THREATS: usize = 3;
/// A last-known mark starts at this alpha and fades linearly to nothing over [`LAST_KNOWN_FADE`] seconds of age.
pub const LAST_KNOWN_ALPHA: f32 = 0.8;
pub const LAST_KNOWN_FADE: f32 = 20.0;
/// A cop's mark floats this far above the car's height, and the helicopter's this far, metres. **[ours]**
pub const COP_LIFT: f32 = 1.5;
pub const HELICOPTER_LIFT: f32 = 25.0;

/// What a threat indicator points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreatKind {
    /// A cop that has the player in sight now.
    Cop,
    /// The helicopter, while it has the player in sight.
    Helicopter,
    /// Where the police last saw the player.
    LastKnown,
}

/// How an indicator is drawn: solid for a confirmed threat, hollow for the last-known place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreatStyle {
    Solid,
    Hollow,
}

impl ThreatKind {
    pub fn style(self) -> ThreatStyle {
        match self {
            ThreatKind::Cop | ThreatKind::Helicopter => ThreatStyle::Solid,
            ThreatKind::LastKnown => ThreatStyle::Hollow,
        }
    }
}

/// One indicator before it is placed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Threat {
    pub kind: ThreatKind,
    /// Where the mark floats, render space.
    pub position: [f32; 3],
    /// From the car, across the ground, metres.
    pub distance: f32,
    /// Multiplies the colour's alpha, on top of `[custom_hud] opacity`.
    pub alpha: f32,
}

/// A threat placed on screen: on its place inside the view, or as an edge arrow pointing along `arrow`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedThreat {
    pub threat: Threat,
    pub at: Vec2,
    pub arrow: Option<Vec2>,
}

/// The alpha of a last-known mark `age_secs` old; `None` once it has faded out.
pub fn last_known_alpha(age_secs: f32) -> Option<f32> {
    if age_secs.is_nan() || age_secs >= LAST_KNOWN_FADE {
        return None;
    }
    Some(LAST_KNOWN_ALPHA * (1.0 - age_secs.max(0.0) / LAST_KNOWN_FADE))
}

/// The police indicators of the frame: the confirmed threats, nearest first and at most [`MAX_THREATS`], or with none
/// confirmed the fading last-known place. Nothing outside a chase, with `pursuit_info` off, or without a car.
pub fn threats(hud: &HudState, settings: &HudSettings) -> Vec<Threat> {
    if hud.pursuit_phase == PursuitPhase::None || !settings.pursuit_info {
        return Vec::new();
    }
    let (Some(car), Some(car_z)) = (hud.minimap.as_ref(), hud.world_marks.car_z) else { return Vec::new() };
    let intel = &hud.police_intel;
    let at = |kind: ThreatKind, place: [f32; 2], lift: f32, alpha: f32| Threat {
        kind,
        position: [place[0], place[1], car_z + lift],
        distance: across(car.position, [place[0], place[1], 0.0]),
        alpha,
    };
    let cops = intel.sightings.iter().map(|&place| at(ThreatKind::Cop, place, COP_LIFT, 1.0));
    let helicopter = intel
        .helicopter
        .filter(|helicopter| helicopter.in_sight)
        .and_then(|helicopter| helicopter.position)
        .map(|place| at(ThreatKind::Helicopter, place, HELICOPTER_LIFT, 1.0));
    let mut confirmed: Vec<Threat> = cops.chain(helicopter).collect();
    if !confirmed.is_empty() {
        confirmed.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        confirmed.truncate(MAX_THREATS);
        return confirmed;
    }
    // The last-known place is shown only while nobody sees the player, as on the map (`fair-knowledge.md` rule 4).
    if !matches!(hud.pursuit_phase, PursuitPhase::Search | PursuitPhase::Cooldown) {
        return Vec::new();
    }
    let Some(last) = intel.last_known else { return Vec::new() };
    let Some(alpha) = last_known_alpha(last.age_secs) else { return Vec::new() };
    vec![at(ThreatKind::LastKnown, last.position, COP_LIFT, alpha)]
}

/// Places `threats` through `camera` on the canvas of `window`: a mark where one projects inside the view (more than
/// `margin` from its edge), else an edge arrow on the ellipse of `edge` (`project::edge_rect`).
pub fn place(threats: Vec<Threat>, camera: &HudCamera, window: Rect, edge: Rect, margin: f32) -> Vec<PlacedThreat> {
    threats
        .into_iter()
        .filter_map(|threat| match project::place(camera, window, threat.position, margin)? {
            Placement::OnScreen(at) => Some(PlacedThreat { threat, at, arrow: None }),
            Placement::OffScreen(direction) => {
                Some(PlacedThreat { threat, at: project::edge_point(edge, direction), arrow: Some(direction) })
            }
        })
        .collect()
}
