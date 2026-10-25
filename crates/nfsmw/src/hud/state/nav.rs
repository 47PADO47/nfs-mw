//! What the HUD knows for navigation: the GPS route as a polyline with its next turn, and the places on the map (race
//! starts, safe houses, shops). The world fills it (`scenes/world/gps/hud.rs`, `scenes/world/hud_nav.rs`).
//!
//! The next turn is read from the route's junctions: where one road segment of the route meets the next, the angle
//! between the last piece of the one and the first piece of the other. A bend inside a segment is the road's own
//! curve, which the player follows anyway, so it is never a turn.

use nfsmw_data::gameplay::RaceType;

/// A junction turns the route when its angle is at least this, degrees.
pub const TURN_DEGREES: f32 = 35.0;
/// A junction turns the route back when its angle is at least this, degrees.
pub const U_TURN_DEGREES: f32 = 150.0;

/// The GPS route the player follows.
#[derive(Debug, Clone, PartialEq)]
pub struct NavRoute {
    /// The route from its start to its end, render space (x, y), metres.
    pub path: Vec<[f32; 2]>,
    /// The point the route leads to, render space (x, y).
    pub target: [f32; 2],
    /// The next turn ahead of the car, or the way straight on to the end; `None` for a route too short to have one.
    pub next_turn: Option<NextTurn>,
}

/// A cue for the next turn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NextTurn {
    pub direction: TurnDirection,
    /// Metres along the route to the turn (to the route's end for [`TurnDirection::Straight`]).
    pub distance_m: f32,
}

/// Which way the next turn goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnDirection {
    Left,
    Right,
    /// No turn before the end of the route.
    Straight,
    UTurn,
}

/// What a place on the map is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavMarkerKind {
    /// A race start, of the race's type.
    Race(RaceType),
    SafeHouse,
    AutoShop,
    CarLot,
}

/// A place on the map the player can go to.
#[derive(Debug, Clone, PartialEq)]
pub struct NavMarker {
    pub kind: NavMarkerKind,
    /// Render space (x, y), metres.
    pub position: [f32; 2],
    /// The career event id of a race (for example `10.2.1`); `None` for the places, whose kind names them.
    pub name: Option<String>,
    /// The player can enter it: a race the Blacklist has opened (or does not list), and every place the map shows.
    pub discovered: bool,
}

/// The next turn of a route `path` for a car at `from`, both render space. `junctions` are the indices in `path` of the
/// points where one road segment meets the next. The car is placed at the nearest point of the route; the first junction
/// ahead of it whose angle reaches [`TURN_DEGREES`] is the turn, and without one the cue is straight on to the end.
/// `None` for a path of fewer than two points.
pub fn next_turn(path: &[[f32; 2]], junctions: &[usize], from: [f32; 2]) -> Option<NextTurn> {
    if path.len() < 2 {
        return None;
    }
    let (piece, along) = nearest_piece(path, from);
    let mut distance = length(path[piece], path[piece + 1]) - along;
    for at in piece + 1..path.len() - 1 {
        if junctions.contains(&at)
            && let Some(direction) = turn_at(path[at - 1], path[at], path[at + 1])
        {
            return Some(NextTurn { direction, distance_m: distance.max(0.0) });
        }
        distance += length(path[at], path[at + 1]);
    }
    Some(NextTurn { direction: TurnDirection::Straight, distance_m: distance.max(0.0) })
}

/// The turn at `at` between the piece from `before` and the piece to `after`: `None` below [`TURN_DEGREES`] (or for a
/// piece of no length). Render space is x forward, y left, z up, so a counter-clockwise turn is to the left.
fn turn_at(before: [f32; 2], at: [f32; 2], after: [f32; 2]) -> Option<TurnDirection> {
    let a = [at[0] - before[0], at[1] - before[1]];
    let b = [after[0] - at[0], after[1] - at[1]];
    if a[0].hypot(a[1]) <= f32::EPSILON || b[0].hypot(b[1]) <= f32::EPSILON {
        return None;
    }
    let cross = a[0] * b[1] - a[1] * b[0];
    let dot = a[0] * b[0] + a[1] * b[1];
    let degrees = cross.atan2(dot).to_degrees();
    if degrees.abs() >= U_TURN_DEGREES {
        return Some(TurnDirection::UTurn);
    }
    if degrees.abs() < TURN_DEGREES {
        return None;
    }
    match degrees > 0.0 {
        true => Some(TurnDirection::Left),
        false => Some(TurnDirection::Right),
    }
}

/// The piece of `path` nearest `point` (its first point's index) and how far along it the nearest point lies, metres.
fn nearest_piece(path: &[[f32; 2]], point: [f32; 2]) -> (usize, f32) {
    let mut best = (0, 0.0, f32::INFINITY);
    for piece in 0..path.len() - 1 {
        let (a, b) = (path[piece], path[piece + 1]);
        let d = [b[0] - a[0], b[1] - a[1]];
        let span = d[0] * d[0] + d[1] * d[1];
        let t = match span > 0.0 {
            true => (((point[0] - a[0]) * d[0] + (point[1] - a[1]) * d[1]) / span).clamp(0.0, 1.0),
            false => 0.0,
        };
        let on = [a[0] + d[0] * t, a[1] + d[1] * t];
        let gap = length(on, point);
        if gap < best.2 {
            best = (piece, t * span.sqrt(), gap);
        }
    }
    (best.0, best.1)
}

fn length(a: [f32; 2], b: [f32; 2]) -> f32 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}

#[cfg(test)]
#[path = "nav_tests.rs"]
mod tests;
