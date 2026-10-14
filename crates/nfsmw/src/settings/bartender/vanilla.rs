//! The vanilla pursuit values the settings start from. The per-level rows are the install-verified ones of
//! `docs/specs/ai-pursuit-heat.md` (section 2 and section 3) and `docs/specs/ai-pursuit.md` (section 5.1 and the constants
//! table); the rewrite has used them since the police were built. A file that sets nothing is these values.

/// Seconds of heat growth per level, 1 to 10 (`TimePerHeatLevel`, bin-independent values; `ai-pursuit-heat.md` 3.2).
pub const TIME_PER_LEVEL: [f32; 10] = [600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 600.0, 780.0, 820.0, 2000.0];

/// The speed under which the player is busted, km/h, per heat level 1 to 10 (`BustSpeed`, `ai-pursuit-heat.md` 2.2).
pub const BUST_SPEED_KPH: [f32; 10] = [9.0, 12.0, 19.0, 20.0, 25.0, 25.0, 19.0, 5.0, 5.0, 5.0];

/// The fine of each infraction, in the order of `blackbox_pursuit::Infraction::ALL` (`infractions`, install-verified).
pub const FINES: [u32; 8] = [150, 350, 1000, 350, 300, 100, 300, 75];

/// The heat a chase keeps after an escape, as a share of the heat it had (`EvadeSuccessHeatAdjust` of the default
/// collection, `ai-pursuit-heat.md` 3.3).
pub const EVADE_HEAT_ADJUST: f32 = 0.95;

/// Cop cars chasing at once, at most. The stock car loader caps spawns at 8 (`ai-pursuit-heat.md` 2.7 and the mod
/// notes); the rewrite keeps 8 until a config says otherwise.
pub const MAX_COPS: u32 = 8;

/// Seconds past which the busted timer busts the player (`kBustedTimeout`, `ai-pursuit.md` 5.1).
pub const BUST_TIMEOUT: f32 = 5.0;

/// Horizontal reach of a height-matched cop, metres, while the player is not flashing (`kBustedCopDistance`).
pub const BUST_REACH: f32 = 15.0;

/// Maximum height difference, metres, between a cop and the player for the cop to count (`ai-pursuit.md` 5.1).
pub const HEIGHT_GATE: f32 = 1.5;
