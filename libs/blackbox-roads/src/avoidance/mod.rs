//! Rules for a traffic car held up by another car in its lane: how long it waits, when it may move to the lane next
//! to it, how the sideways move blends in, when it sounds its horn or counts as stuck, and when it waits for an
//! oncoming gap at a crossing. Pure numbers and lane indices; the navigator and the world do the moving.
//! Rewrite extensions, none in the original (spec `docs/specs/ai-traffic-obstacles.md` §10).

mod blend;
mod blocked;
mod change;
mod crossing;
mod gap;

pub use blend::{BLEND_SECONDS, Blend, MIN_BLEND_SPEED, blend_length, smoothstep};
pub use blocked::{Blocked, CHANGE_AFTER, HONK_AFTER, SLOW, STANDING, STUCK_AFTER};
pub use change::{Command, LaneChange, Phase, RETURN_AFTER, Situation};
pub use crossing::{Crossing, GIVE_UP_AFTER, Gate, WAIT_FOR_GAP, time_to_closest};
pub use gap::{AHEAD_EXTRA, AHEAD_SECONDS, BEHIND, Other, adjacent_lanes, lane_clear};
