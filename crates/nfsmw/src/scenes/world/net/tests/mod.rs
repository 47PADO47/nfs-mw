//! The host's tests. `input.rs` runs everywhere; `installed*.rs` need the installed game (`NFSMW_GAME_DIR`) and are
//! ignored without it, as the world's other installed tests are.

mod bench;
mod input;
mod installed;
mod installed_events;
mod installed_race;
