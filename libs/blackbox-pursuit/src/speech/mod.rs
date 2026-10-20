//! What the cops and dispatch say during a chase, as a pure state machine: the opening exchange, the strategy
//! commentary, backup requests, the lost-contact lines, roadblocks, the observer's remarks on crashes and stunts, the
//! arrest bullhorn, heat jumps, and the 911 call. It reads a [`Snapshot`] of the pursuit and its cars plus the
//! [`Notice`]s since the last step, asks the radio queue ([`Channel`]) whether it is free, and returns
//! [`Request`]s: a [`Line`], a speaker (a voice number) and typed [`Params`]. It plays no audio and knows no
//! game's names: the caller maps lines and parameters to its speech events, and gives the voice [`Layout`] and the
//! [`Tune`] thresholds.
//!
//! Spec: `docs/specs/ai-pursuit-speech.md` (the original's `SoundAI`, `PursuitFlow`, `StrategyFlow`, `RoadblockFlow`,
//! `Observer` and the actors) and `docs/specs/ai-pursuit-speech-events.md` (the event tables).

mod actors;
mod config;
mod cx;
mod flows;
pub mod impact;
mod input;
mod line;
mod machine;
mod notices;
mod observer;
mod observer_said;
mod output;
mod params;
mod pursuit_flow;
mod pursuit_spotter;
mod roadblock_flow;
mod say;
mod say_dispatch;
mod say_events;
mod state;
mod strategy_flow;
mod sync_actors;
mod sync_player;
mod sync_pursuit;
#[cfg(test)]
mod tests;

pub use actors::{Actor, Pick, Roster};
pub use config::{Layout, Pool, Tune};
pub use cx::Cx;
pub use input::{
    Channel, CopInfo, HeliInfo, Impact, Level, Notice, PlayerInfo, PursuitInfo, RoadblockHit, RoadblockInfo, Snapshot,
    Strategy,
};
pub use line::{Line, Speaker};
pub use machine::Machine;
pub use output::{Effect, Output, Request, Talker};
pub use params::*;
pub use state::{Focus, PursuitState, Quadrant, Rng};
