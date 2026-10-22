//! MP-33 collision modes: whether two bodies collide at a tick. Only racer-to-racer contact changes with the mode;
//! traffic, police and the world always collide.
//!
//! - `Full`: racers collide.
//! - `Ghost`: racers pass through each other.
//! - `GhostStart`: racers pass through each other for the first [`GHOST_START_TICKS`] after the start, then collide.
//!
//! A body marked `ghost` (a reconnecting player's parked car, MP-51) collides with the world only, in every mode.
//! Before the start (the grid and the countdown) the caller passes 0 ticks since the start. Values **[ours]**: the
//! ghost start lasts 8 s (features.md says "the first seconds").

use crate::message::CollisionMode;
use nfsmw_sim::TICK_HZ;

/// How long a ghost start lasts: eight seconds.
pub const GHOST_START_TICKS: u32 = 8 * TICK_HZ;

/// What a body is, for the collision rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyKind {
    /// A player's car.
    Racer,
    Traffic,
    Police,
    /// The road, walls and props.
    World,
}

/// One side of a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Body {
    pub kind: BodyKind,
    /// A car that collides with the world only.
    pub ghost: bool,
}

impl Body {
    /// A body of `kind` that is not a ghost.
    pub fn solid(kind: BodyKind) -> Self {
        Self { kind, ghost: false }
    }
}

/// Whether `a` and `b` collide under `mode`, `since_start` ticks after the race start (0 before it; free roam passes
/// `u32::MAX`).
pub fn pair_collides(mode: CollisionMode, a: Body, b: Body, since_start: u32) -> bool {
    if a.ghost || b.ghost {
        return a.kind == BodyKind::World || b.kind == BodyKind::World;
    }
    if a.kind != BodyKind::Racer || b.kind != BodyKind::Racer {
        return true;
    }
    match mode {
        CollisionMode::Full => true,
        CollisionMode::Ghost => false,
        CollisionMode::GhostStart => since_start >= GHOST_START_TICKS,
    }
}
