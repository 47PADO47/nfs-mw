//! The speech events: the AttribSys class `speech` (one collection per phrase the police dispatch can say) and the
//! `speechtune` collection. They hold the *when* of speech (priority, how long a request stays valid, how often it may
//! repeat); the *what* is in the sound files. Spec: `docs/specs/speech.md`.

use blackbox_attrib::{CollectionRef, Database, Value};

/// One speech event's scheduling data. Field names are the class's. Times are seconds, speeds are miles per hour.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechEvent {
    /// `CollectionName`: the event's name (`StaticRoadblock_CallForRB`).
    pub name: String,
    /// `SpeechID`: the event number of the speech database (`copspeech.evt`).
    pub id: u32,
    pub priority: i32,
    /// An interrupting event jumps the queue and may cut the line being said (`priority` counts 100 higher).
    pub interrupt: bool,
    /// Whether another interrupting event may cut this one off while it is being said.
    pub interruptable: bool,
    /// Seconds a request waits before it is dropped.
    pub expiry: f32,
    /// Seconds a request waits before it may be said at all.
    pub init_delay: f32,
    /// Seconds that must pass between two sayings of the event.
    pub interval: f32,
    /// How often the event may be said in all; negative for no limit.
    pub max_playback: i32,
    /// Silence needed since the last line before this one starts.
    pub dead_air: f32,
    /// Silence enforced after this event, for the lines behind it.
    pub enforce_dead_air: f32,
    /// With `dep_follow`: how recently one of those events must have been said (0 means "the last one said").
    pub back_time: f32,
    /// Distance beyond which the speaking unit is not heard.
    pub culling_range: f32,
    /// Dropped requests are not thinned out at random (see [`SpeechTune`]).
    pub do_not_dropout: bool,
    pub min_heat: i32,
    pub max_heat: i32,
    pub min_player_speed: f32,
    pub max_player_speed: f32,
    /// `OnScreenOnly`: only said while the speaking unit is on screen.
    pub on_screen_only: bool,
    /// `reqLOS`: only said while the speaking unit has a line of sight to the player.
    pub require_line_of_sight: bool,
    /// `Clarity` (0 to 10): how much radio filtering the line gets. Not used yet.
    pub clarity: u32,
    /// Whether the line is panned to the speaking unit. Not used yet.
    pub pan: bool,
    /// Whether the radio click plays around the line. Not used yet.
    pub radio_chirp: bool,
    /// `DepFollow`: events (by `SpeechID`) one of which must have been said just before.
    pub dep_follow: Vec<u32>,
    /// `RecallList`: events (by `SpeechID`) that are withdrawn from the queue when this one is requested.
    pub recall: Vec<u32>,
}

impl Default for SpeechEvent {
    /// The values of the database's `default` collection, which the others inherit.
    fn default() -> Self {
        Self {
            name: String::new(),
            id: 0,
            priority: 50,
            interrupt: false,
            interruptable: true,
            expiry: 5.0,
            init_delay: 0.0,
            interval: 20.0,
            max_playback: -1,
            dead_air: 0.0,
            enforce_dead_air: 2.0,
            back_time: 0.0,
            culling_range: 300.0,
            do_not_dropout: false,
            min_heat: 0,
            max_heat: 10,
            min_player_speed: 0.0,
            max_player_speed: 550.0,
            on_screen_only: false,
            require_line_of_sight: false,
            clarity: 10,
            pan: false,
            radio_chirp: true,
            dep_follow: Vec::new(),
            recall: Vec::new(),
        }
    }
}

fn flag(c: CollectionRef<'_>, field: &str) -> bool {
    c.get_bool(field).unwrap_or(false)
}

fn float(c: CollectionRef<'_>, field: &str) -> f32 {
    c.get_f32(field).unwrap_or(0.0)
}

fn int(c: CollectionRef<'_>, field: &str) -> i32 {
    c.get_i32(field).unwrap_or(0)
}

/// The `SpeechID`s of the collections an array field refers to.
fn ids_of(db: &Database, c: CollectionRef<'_>, field: &str) -> Vec<u32> {
    let Some(Value::Array(items)) = c.get(field) else { return Vec::new() };
    let refs = items.iter().filter_map(Value::as_ref_spec).filter_map(|r| db.resolve(r));
    refs.filter_map(|r| r.get_u32("SpeechID")).collect()
}

fn event(db: &Database, c: CollectionRef<'_>) -> Option<SpeechEvent> {
    Some(SpeechEvent {
        name: c.get_str("CollectionName").unwrap_or_default().trim().to_owned(),
        id: c.get_u32("SpeechID")?,
        priority: int(c, "priority"),
        interrupt: flag(c, "interrupt"),
        interruptable: flag(c, "Interruptable"),
        expiry: float(c, "expiry"),
        init_delay: float(c, "InitDelay"),
        interval: float(c, "Interval"),
        max_playback: c.get_i32("MaxPlayback").unwrap_or(-1),
        dead_air: float(c, "DeadAir"),
        enforce_dead_air: float(c, "EnforceDeadAir"),
        back_time: float(c, "BackTime"),
        culling_range: float(c, "CullingRange"),
        do_not_dropout: flag(c, "DoNotDropout"),
        min_heat: int(c, "MinHeat"),
        max_heat: int(c, "MaxHeat"),
        min_player_speed: float(c, "MinPlayerSpeed"),
        max_player_speed: float(c, "MaxPlayerSpeed"),
        on_screen_only: flag(c, "OnScreenOnly"),
        require_line_of_sight: flag(c, "reqLOS"),
        clarity: c.get_u32("Clarity").unwrap_or(0),
        pan: flag(c, "Pan"),
        radio_chirp: flag(c, "RadioChirp"),
        dep_follow: ids_of(db, c, "DepFollow"),
        recall: ids_of(db, c, "RecallList"),
    })
}

/// Every speech event of the database, in the database's order; empty when it has none.
pub fn events(db: &Database) -> Vec<SpeechEvent> {
    db.collections_of("speech").filter_map(|c| event(db, c)).collect()
}

/// The tuning shared by all speech: how requests are thinned out as a pursuit goes on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeechTune {
    /// `SpeechDropoffRamp`: from the first time (seconds into a pursuit) a request is said with a probability that
    /// falls linearly from 1; at the second it reaches 0.
    pub dropoff_ramp: [f32; 2],
}

impl Default for SpeechTune {
    fn default() -> Self {
        Self { dropoff_ramp: [f32::MAX, f32::MAX] }
    }
}

/// The `speechtune` collection, or the default (no thinning) when the database has none.
pub fn tune(db: &Database) -> SpeechTune {
    let Some(c) = db.collections_of("speechtune").next() else { return SpeechTune::default() };
    SpeechTune { dropoff_ramp: c.get_vector2("SpeechDropoffRamp").unwrap_or(SpeechTune::default().dropoff_ramp) }
}
