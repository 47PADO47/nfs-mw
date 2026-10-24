//! The marks on the overlay, as plain data in world space: the places, the race, the route's end and what the player
//! knows of the police. `draw.rs` turns them into shapes.
//!
//! **Fair knowledge** (`docs/specs/custom-hud/fair-knowledge.md`). The police marks come from `HudState::police_intel`
//! alone, never from the cop blips, which follow the original minimap's rules:
//!
//! - a cop is marked only while it has the player in sight (`sightings`);
//! - the helicopter only while it has the player in sight;
//! - where the police last saw the player is a hollow mark, only once no one sees the player (Search and Cooldown),
//!   fading from [`LAST_KNOWN_ALPHA`] to nothing over [`LAST_KNOWN_SECS`];
//! - a roadblock only once dispatch has reported it.

use crate::hud::{CHECKPOINT, HudState, LastKnown, NavMarkerKind, PursuitPhase, RACE_SPEEDTRAP, RACER};

/// How long the last-known mark fades for, seconds, and its alpha when fresh.
pub const LAST_KNOWN_SECS: f32 = 20.0;
pub const LAST_KNOWN_ALPHA: f32 = 0.8;
/// A checkpoint blip this close to the next gate (metres) is that gate, already marked.
const SAME_PLACE_M: f32 = 2.0;

/// What a mark stands for. Each has its own shape and token colour (`draw.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkKind {
    /// The gate the player drives to next.
    NextCheckpoint,
    Checkpoint,
    SpeedTrap,
    Racer,
    /// The end of the GPS route.
    Target,
    /// A discovered race start.
    Race,
    SafeHouse,
    /// An auto shop or a car lot.
    Shop,
    /// A cop that has the player in sight.
    Cop,
    /// The helicopter, while it has the player in sight.
    Helicopter,
    /// Where the police last saw the player.
    LastKnown,
    /// A reported roadblock.
    Roadblock,
}

impl MarkKind {
    /// Whether the mark stays in view at the overlay's edge when its place is outside it.
    pub fn pinned(self) -> bool {
        matches!(self, MarkKind::NextCheckpoint | MarkKind::Target)
    }
}

/// One mark: what it is, where (render space, metres) and its own alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    pub kind: MarkKind,
    pub at: [f32; 2],
    pub alpha: f32,
}

impl Mark {
    fn new(kind: MarkKind, at: [f32; 2]) -> Self {
        Self { kind, at, alpha: 1.0 }
    }
}

/// The marks of `hud`, back to front: the places, the race, the route's end, the police.
pub fn marks(hud: &HudState) -> Vec<Mark> {
    let mut out = Vec::new();
    places(hud, &mut out);
    race(hud, &mut out);
    if let Some(route) = &hud.route {
        out.push(Mark::new(MarkKind::Target, route.target));
    }
    police(hud, &mut out);
    out
}

/// The discovered places: race starts, safe houses and shops.
fn places(hud: &HudState, out: &mut Vec<Mark>) {
    for marker in hud.markers.iter().filter(|marker| marker.discovered) {
        let kind = match marker.kind {
            NavMarkerKind::Race(_) => MarkKind::Race,
            NavMarkerKind::SafeHouse => MarkKind::SafeHouse,
            NavMarkerKind::AutoShop | NavMarkerKind::CarLot => MarkKind::Shop,
        };
        out.push(Mark::new(kind, marker.position));
    }
}

/// The race: the next gate, the checkpoint and speed-trap blips, the rivals.
fn race(hud: &HudState, out: &mut Vec<Mark>) {
    let next = hud.race_info.as_ref().and_then(|race| race.next_gate).map(|gate| [gate[0], gate[1]]);
    if let Some(at) = next {
        out.push(Mark::new(MarkKind::NextCheckpoint, at));
    }
    for blip in &hud.blips {
        let kind = match blip.style {
            style if style == CHECKPOINT => MarkKind::Checkpoint,
            style if style == RACE_SPEEDTRAP => MarkKind::SpeedTrap,
            style if style == RACER => MarkKind::Racer,
            _ => continue,
        };
        let is_next = next.is_some_and(|at| distance(at, blip.position) < SAME_PLACE_M);
        if kind == MarkKind::Checkpoint && is_next {
            continue;
        }
        out.push(Mark::new(kind, blip.position));
    }
}

/// What the player knows of the police; see the module.
fn police(hud: &HudState, out: &mut Vec<Mark>) {
    let intel = &hud.police_intel;
    if let Some(last) = intel.last_known
        && let Some(alpha) = last_known_alpha(hud.pursuit_phase, last)
    {
        out.push(Mark { kind: MarkKind::LastKnown, at: last.position, alpha });
    }
    if intel.roadblock_reported {
        out.extend(intel.roadblocks.iter().map(|at| Mark::new(MarkKind::Roadblock, *at)));
    }
    let helicopter = intel.helicopter.filter(|heli| heli.in_sight).and_then(|heli| heli.position);
    out.extend(helicopter.map(|at| Mark::new(MarkKind::Helicopter, at)));
    out.extend(intel.sightings.iter().map(|at| Mark::new(MarkKind::Cop, *at)));
}

/// The alpha of the last-known mark in `phase`: only while no one sees the player (Search, Cooldown), fading linearly
/// from [`LAST_KNOWN_ALPHA`] at age 0 to nothing at [`LAST_KNOWN_SECS`]; `None` when it is not drawn.
pub fn last_known_alpha(phase: PursuitPhase, last: LastKnown) -> Option<f32> {
    if !matches!(phase, PursuitPhase::Search | PursuitPhase::Cooldown) {
        return None;
    }
    if last.age_secs.is_nan() || last.age_secs >= LAST_KNOWN_SECS {
        return None;
    }
    let age = last.age_secs.max(0.0);
    Some(LAST_KNOWN_ALPHA * (1.0 - age / LAST_KNOWN_SECS))
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}
