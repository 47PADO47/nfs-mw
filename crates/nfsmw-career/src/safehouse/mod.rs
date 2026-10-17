//! The safe house, native side: entering, the menu and its sequences, leaving, the respawn marker and heat
//! between worlds (`docs/specs/career-safehouse.md`, `docs/specs/career-safehouse-2.md`, with the in-place
//! updates of `career-flow.md` section 7 and `career-scripts.md` section 4).
//!
//! Pure state and rules, no I/O: the caller owns the world, the screens, the marker inventory and the data lookups
//! (collection existence, name hashes, event completion) and passes them in as inputs or closures.
//!
//! - [`zone`]: `ZoneType` classification and the per-frame entry permissions.
//! - [`engage`]: the engage button decision, the cool-down path and the evade bookkeeping that follows.
//! - [`respawn`]: the respawn marker keys, the selector flag and the position and heading helpers.
//! - [`first_visit`]: the stage 16 to 15 step of the first visit.
//! - [`menu`] and [`stepper`]: the menu items and the Blacklist sequence order.
//! - [`resume`]: Resume Free Roam, the pending prologue event and the roaming restart effects.
//! - [`heat`]: heat written at the world unload and read at the car spawn, and the evade, win and milestone
//!   factors, with the overwrite quirk made explicit.
//! - [`autosave`]: the trigger and the gate.
//! - [`phone`]: the phone flush conditions in the safe house and in free roam.

pub mod autosave;
pub mod context;
pub mod engage;
pub mod first_visit;
pub mod heat;
pub mod menu;
pub mod phone;
pub mod respawn;
pub mod resume;
pub mod stepper;
pub mod zone;

pub use autosave::{AutosaveInputs, AutosaveState, AutosaveTrigger, autosave_possible, trigger_posts_event};
pub use context::FrontEndContext;
pub use engage::{
    CoolDownEnd, EngageDecision, EngageInput, cooldown_end, engage, finish_cooldown_pursuit,
    finish_cooldown_pursuit_default, results_screen_closed,
};
pub use first_visit::{FIRST_VISIT_MOVIE, FirstVisit, bio_screen_opened};
pub use heat::{
    DEFAULT_EVADE_HEAT_FACTOR, DEFAULT_MILESTONE_HEAT_FACTOR, DEFAULT_RACE_WIN_HEAT_FACTOR, SPAWN_HEAT_FLOOR,
    UNLOAD_HEAT_CAP, apply_heat_factor, spawn_heat, unload_world,
};
pub use menu::{MenuBindings, MenuItem, MenuTarget, bindings, visible_items};
pub use phone::{free_roam_flush_context, safe_house_flush_context};
pub use respawn::{
    HEADING_FACTOR, MARKER_HEADING_ATTRIBUTE, MARKER_POSITION_ATTRIBUTE, Respawn, StartSource, ZONE_RESPAWN_ATTRIBUTE,
    career_start_label, engine_position, heading_rotation, start_source,
};
pub use resume::{
    EventProgress, PENDING_NODE_MODE, PROLOGUE_EVENTS, PendingEvent, ResumeOutcome, RoamingRestart, resume_free_roam,
};
pub use stepper::{BlacklistStepper, StepAction};
pub use zone::{EntryPermissions, PursuitPhase, ZoneKind, ZoneOpen};
