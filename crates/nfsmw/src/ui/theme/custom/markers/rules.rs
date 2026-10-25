//! Which world markers show and how (`docs/specs/custom-hud/world-markers.md` sections 2, 4, 5 and 6): the candidates
//! the frame's HUD state offers, the priority order and the cap, and the styling by distance and occlusion.
//!
//! Distances are measured across the ground from the car (`HudState::minimap`), in metres.

use std::hash::{DefaultHasher, Hash, Hasher};

use crate::hud::{HudState, MapPosition, NavMarkerKind};
use crate::settings::hud::HudSettings;

/// The most markers on screen at once, edge arrows included.
pub const MAX_MARKERS: usize = 6;
/// A mark is at full alpha and full size within this distance.
pub const FULL_WITHIN: f32 = 50.0;
/// Labels are hidden beyond this distance (an edge arrow keeps its distance label).
pub const LABEL_WITHIN: f32 = 300.0;
/// A mark fades to this alpha, and shrinks to this size, at its kind's range.
pub const FAR_ALPHA: f32 = 0.35;
pub const FAR_SIZE: f32 = 0.6;
/// A checkpoint the static world hides draws at this alpha, without its label.
pub const OCCLUDED_ALPHA: f32 = 0.3;
/// The active checkpoint is emphasised (full alpha, larger) within this distance.
pub const EMPHASIS_WITHIN: f32 = 150.0;
pub const EMPHASIS_SIZE: f32 = 1.25;
/// The active gate dims to [`FAR_ALPHA`] by this distance, but stays: it is the way to go. **[ours]**
pub const GATE_RANGE: f32 = 600.0;
/// Event entrances and other map places are marked within this distance.
pub const ENTRANCE_RANGE: f32 = 300.0;
/// Pursuit breakers are marked within this distance (the scene's sight distance, `scenes/world/hud_marks.rs`).
pub const BREAKER_RANGE: f32 = 150.0;
/// A gate behind the car closer than this is being driven through, not missed.
pub const BEHIND_CAR: f32 = 15.0;
/// A mark floats this far above its place, metres: the gate's centre, or the ground under a place.
pub const GATE_LIFT: f32 = 2.0;
pub const PLACE_LIFT: f32 = 3.0;

/// What a marker marks, in priority order: when more qualify than [`MAX_MARKERS`], the earlier kinds win.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// The next gate of the race.
    ActiveCheckpoint,
    /// The next gate, after a wrong way or with the gate behind the car: an arrow back to it.
    Recovery,
    /// A pursuit breaker in sight during a chase.
    Breaker,
    /// A race start in free roam.
    Entrance,
    /// A safe house, shop or car lot in free roam (with the free-roam extras on).
    Discovered,
}

impl Kind {
    /// The distance at which the kind's mark has faded to [`FAR_ALPHA`]; past it, a kind that does not guide is gone.
    pub fn range(self) -> f32 {
        match self {
            Kind::ActiveCheckpoint | Kind::Recovery => GATE_RANGE,
            Kind::Breaker => BREAKER_RANGE,
            Kind::Entrance | Kind::Discovered => ENTRANCE_RANGE,
        }
    }

    /// Whether the marker guides the player: off screen or behind the camera it becomes an edge arrow, and behind
    /// geometry it stays, dimmed. A kind that does not guide is hidden in both cases.
    pub fn guides(self) -> bool {
        matches!(self, Kind::ActiveCheckpoint | Kind::Recovery)
    }

    /// Whether the kind's mark carries a label at all (breakers and discovered places are icons only).
    pub fn labelled(self) -> bool {
        !matches!(self, Kind::Breaker | Kind::Discovered)
    }
}

/// What a marker's label names.
#[derive(Clone, Debug, PartialEq)]
pub enum Name {
    None,
    /// Gate `gate` of `gates`.
    Gate {
        gate: u32,
        gates: u32,
    },
    /// A map place, with its career event id for a race.
    Place(NavMarkerKind, Option<String>),
}

/// A marker the frame offers, before it is placed on screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// Stable across frames for the same target: the label smoothing is kept per id.
    pub id: u64,
    pub kind: Kind,
    /// Where the mark floats, render space.
    pub position: [f32; 3],
    /// From the car, across the ground, metres.
    pub distance: f32,
    /// The static world hides it from the camera.
    pub occluded: bool,
    pub name: Name,
}

/// How a placed marker draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// Multiplies the colours' alpha, on top of `[custom_hud] opacity`.
    pub alpha: f32,
    /// Multiplies the mark's size, on top of the HUD scale.
    pub size: f32,
    /// The label shows.
    pub label: bool,
}

/// The markers the HUD state offers this frame: the next gate (as a recovery arrow while `recovering`, or when it is
/// behind the car), the map places near the car and the breakers in sight. None without a car on the map.
pub fn gather(hud: &HudState, settings: &HudSettings, recovering: bool) -> Vec<Candidate> {
    let Some(car) = hud.minimap.as_ref() else { return Vec::new() };
    let mut out: Vec<Candidate> = gate(hud, car, recovering).into_iter().collect();
    for entrance in &hud.world_marks.entrances {
        let Some(marker) = hud.markers.get(entrance.marker) else { continue };
        let kind = match marker.kind {
            NavMarkerKind::Race(_) => Kind::Entrance,
            _ => Kind::Discovered,
        };
        if kind == Kind::Discovered && !settings.free_roam_extras {
            continue;
        }
        let at = entrance.point.position;
        out.push(Candidate {
            id: id(kind, at),
            kind,
            position: lifted(at, PLACE_LIFT),
            distance: across(car.position, at),
            occluded: entrance.point.occluded,
            name: Name::Place(marker.kind, marker.name.clone()),
        });
    }
    for breaker in &hud.world_marks.breakers {
        let at = breaker.position;
        out.push(Candidate {
            id: id(Kind::Breaker, at),
            kind: Kind::Breaker,
            position: lifted(at, PLACE_LIFT),
            distance: across(car.position, at),
            occluded: breaker.occluded,
            name: Name::None,
        });
    }
    out
}

/// The next gate's marker, if the race has one.
fn gate(hud: &HudState, car: &MapPosition, recovering: bool) -> Option<Candidate> {
    let race = hud.race_info.as_ref()?;
    let at = race.next_gate?;
    let distance = across(car.position, at);
    let behind = distance > BEHIND_CAR && behind_car(car, at);
    let kind = match recovering || behind {
        true => Kind::Recovery,
        false => Kind::ActiveCheckpoint,
    };
    let name = match race.gate {
        Some(gate) => Name::Gate { gate, gates: race.gates },
        None => Name::None,
    };
    // One id for the gate whichever way it draws, so its label glides from the mark to the arrow and back.
    let id = id(Kind::ActiveCheckpoint, [race.gate.unwrap_or(0) as f32, 0.0, 0.0]);
    let occluded = hud.world_marks.gate_occluded;
    Some(Candidate { id, kind, position: lifted(at, GATE_LIFT), distance, occluded, name })
}

/// Whether `at` is behind the car: on the far side of the line across its heading.
fn behind_car(car: &MapPosition, at: [f32; 3]) -> bool {
    let to = [at[0] - car.position[0], at[1] - car.position[1]];
    to[0] * car.heading[0] + to[1] * car.heading[1] < 0.0
}

/// The distance from `car` to `at` across the ground (x, y), metres.
pub fn across(car: [f32; 2], at: [f32; 3]) -> f32 {
    (at[0] - car[0]).hypot(at[1] - car[1])
}

/// `at` raised by `lift` metres.
fn lifted(at: [f32; 3], lift: f32) -> [f32; 3] {
    [at[0], at[1], at[2] + lift]
}

/// A stable id for a marker of `kind` at `at`.
fn id(kind: Kind, at: [f32; 3]) -> u64 {
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    at.map(f32::to_bits).hash(&mut hasher);
    hasher.finish()
}

/// How a marker of `kind` draws at `distance`: full within [`FULL_WITHIN`], fading and shrinking linearly to
/// [`FAR_ALPHA`] and [`FAR_SIZE`] at the kind's range. `None` for a marker that does not show: past its range (unless
/// it guides) or hidden behind geometry (unless it guides; a guiding one dims and loses its label). The active
/// checkpoint is emphasised within [`EMPHASIS_WITHIN`].
pub fn look(kind: Kind, distance: f32, occluded: bool) -> Option<Look> {
    if !kind.guides() && (distance > kind.range() || occluded) {
        return None;
    }
    let t = ((distance - FULL_WITHIN) / (kind.range() - FULL_WITHIN)).clamp(0.0, 1.0);
    let alpha = 1.0 + (FAR_ALPHA - 1.0) * t;
    let size = 1.0 + (FAR_SIZE - 1.0) * t;
    let label = kind.labelled() && distance <= LABEL_WITHIN;
    if occluded {
        return Some(Look { alpha: OCCLUDED_ALPHA, size, label: false });
    }
    if kind == Kind::ActiveCheckpoint && distance <= EMPHASIS_WITHIN {
        return Some(Look { alpha: 1.0, size: size * EMPHASIS_SIZE, label });
    }
    Some(Look { alpha, size, label })
}

/// Keeps the `cap` items first in priority order: by kind (the order of [`Kind`]), then nearest first within a kind.
/// `rank` gives an item's kind and distance.
pub fn select<T>(items: &mut Vec<T>, cap: usize, rank: impl Fn(&T) -> (Kind, f32)) {
    items.sort_by(|a, b| {
        let ((kind_a, distance_a), (kind_b, distance_b)) = (rank(a), rank(b));
        kind_a.cmp(&kind_b).then(distance_a.total_cmp(&distance_b))
    });
    items.truncate(cap);
}
