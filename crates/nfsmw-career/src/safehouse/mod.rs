//! The safe house, native side: entering, the menu and its sequences, leaving, the respawn marker and heat
//! between worlds (`docs/specs/career-safehouse.md`, `docs/specs/career-safehouse-2.md`, with the in-place
//! updates of `career-flow.md` section 7 and `career-scripts.md` section 4).
//!
//! Pure state and rules, no I/O: the caller owns the world, the screens, the marker inventory and the data lookups
//! (collection existence, name hashes, event completion) and passes them in as inputs or closures.
//!
//! - [`zone`]: `ZoneType` classification and the per-frame entry permissions.
//! - [`engage`]: the engage button decision, the cool-down path and the evade bookkeeping that follows.
//! - [`heat`]: heat written at the world unload and read at the car spawn, and the evade, win and milestone
//!   factors, with the overwrite quirk made explicit.

pub mod context;
pub mod engage;
pub mod heat;
pub mod zone;

pub use context::FrontEndContext;
pub use engage::{
    CoolDownEnd, EngageDecision, EngageInput, cooldown_end, engage, finish_cooldown_pursuit,
    finish_cooldown_pursuit_default, results_screen_closed,
};
pub use heat::{
    DEFAULT_EVADE_HEAT_FACTOR, DEFAULT_MILESTONE_HEAT_FACTOR, DEFAULT_RACE_WIN_HEAT_FACTOR, SPAWN_HEAT_FLOOR,
    UNLOAD_HEAT_CAP, apply_heat_factor, spawn_heat, unload_world,
};
pub use zone::{EntryPermissions, PursuitPhase, ZoneKind, ZoneOpen};
