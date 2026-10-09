//! The interactive music: the soundtrack that follows the game.
//!
//! The licensed songs are the radio's ([`super::radio`]). This module adds what the original's PathFinder graph
//! (`MW_Music.mpf`) does besides songs: the **pursuit music**, four sets of bars that the game steers with a
//! control value, 0 to 127. Spec: `docs/specs/music-graph.md` section 8 and `docs/specs/interactive-music.md`.
//!
//! - [`MusicInput`] is what the game tells it: a pursuit (set, intensity) and whether a race is on. The pursuit and
//!   race code of milestone 7 calls its hooks; until then only the `music` console command sets a state.
//! - [`director`] decides: which set plays, the smoothed control value, and when the songs may return (40 s after a
//!   pursuit).
//! - [`conductor`] carries it out: starts a pursuit track ([`live`] follows the graph bar by bar with the control
//!   value), cross-fades between sets, and tells the radio when it may play.
//!
//! Nothing here has been listened to, and the events of the map (fades, waits, conditionals, variables such as
//! `pursuitid`) are not interpreted: a set is entered at its group head and steered by the transitions alone.

mod conductor;
mod director;
mod glue;
mod kira_stage;
mod live;
mod pursuit;
mod state;
#[cfg(test)]
mod tests;

pub use glue::{Interactive, command};
#[allow(unused_imports)] // for the pursuit and race code of milestone 7
pub use state::{MusicInput, MusicState, Pursuit, pursuit_set_for_heat};
