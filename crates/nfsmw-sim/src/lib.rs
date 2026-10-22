//! The world-simulation seam of the multiplayer server (ADR 0011). A [`WorldSim`] is an authoritative world that advances
//! one 60 Hz physics tick at a time from the input of every seated player and can describe itself as plain entity records.
//! The game's `WorldScene` implements it in `crates/nfsmw`; [`toy::ToyWorld`] implements it for tests and bots. Nothing here
//! knows the renderer, audio, settings or the wire format.

pub use blackbox_net_core::session::PlayerId;
pub use blackbox_vehicle::InputState;

pub mod hash;
pub mod runner;
pub mod toy;

/// Simulation time: the number of 60 Hz physics steps since the session's world started.
pub type Tick = u32;

/// The physics step rate, in steps per second.
pub const TICK_HZ: u32 = 60;

/// A world entity's id in the server's 16-bit id space (ADR 0009 section 1).
pub type EntityId = u16;

/// The input of every seated player for one tick, in player-id order.
#[derive(Default, Debug, Clone)]
pub struct SimInputs {
    seats: Vec<(PlayerId, InputState)>,
}

impl SimInputs {
    /// Store `input` for `id`, replacing that player's earlier entry.
    pub fn set(&mut self, id: PlayerId, input: InputState) {
        match self.seats.binary_search_by(|(seat, _)| seat.cmp(&id)) {
            Ok(index) => self.seats[index].1 = input,
            Err(index) => self.seats.insert(index, (id, input)),
        }
    }

    /// The player's input this tick, or `None` when the player has none (the world holds neutral input).
    pub fn get(&self, id: PlayerId) -> Option<&InputState> {
        let index = self.seats.binary_search_by(|(seat, _)| seat.cmp(&id)).ok()?;
        Some(&self.seats[index].1)
    }

    /// The seated players and their input, in id order.
    pub fn iter(&self) -> impl Iterator<Item = (PlayerId, &InputState)> {
        self.seats.iter().map(|(id, input)| (*id, input))
    }

    pub fn clear(&mut self) {
        self.seats.clear();
    }
}

/// What kind of thing an entity is. The numbers are the wire kinds of `protocol.md` section 6.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EntityKind {
    PlayerCar = 1,
    TrafficCar = 3,
    PoliceCar = 4,
    Helicopter = 5,
    Roadblock = 6,
    MovingProp = 7,
}

/// One world entity as the server replicates it: pose and motion in world metres, plus the game-defined flag bits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityView {
    pub id: EntityId,
    pub kind: EntityKind,
    /// The player who drives it, for a player car.
    pub owner: Option<PlayerId>,
    pub position: [f32; 3],
    /// Unit quaternion `[x, y, z, w]`.
    pub orientation: [f32; 4],
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    /// Car model, prop type or similar, a small game-defined number.
    pub type_id: u16,
    /// Brake lights, siren, broken, signal and so on: game-defined bits.
    pub flags: u32,
}

/// The reduced authoritative state of a player's own car that the server sends back for reconciliation
/// (`protocol.md` section 6.3, own block).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OwnState {
    pub rpm: f32,
    pub gear: u8,
    /// Nitrous left, 0..1.
    pub nitrous: f32,
    /// Speedbreaker meter, 0..1.
    pub boost: f32,
    pub flags: u8,
}

/// What a player asks for when joining the world: ids only, never numbers (MP-30).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerJoin {
    /// The car model's id in the game data.
    pub car_type: u32,
    /// The ids of the installed parts.
    pub parts: Vec<u32>,
    /// Where to appear, when the session picks the spot (a race grid slot, a lobby pad); `None` lets the world choose.
    pub spawn: Option<[f32; 3]>,
}

/// Why the world refused a request.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SimError {
    #[error("player {0:?} is already in the world")]
    AlreadyJoined(PlayerId),
    #[error("unknown car type {0}")]
    UnknownCar(u32),
    #[error("the world has no room for another player")]
    Full,
}

/// A thing that happened during a tick that clients, results or the chat feed need to hear about. Variants may be added;
/// none is renamed or reordered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimEvent {
    /// Two entities touched at `speed` metres per second of relative speed.
    Contact {
        a: EntityId,
        b: EntityId,
        speed: f32,
    },
    /// The server moved a player's car (a reset, a respawn, a teleport).
    Moved {
        player: PlayerId,
    },
    PursuitStarted {
        player: PlayerId,
    },
    PursuitEnded {
        player: PlayerId,
    },
    Busted {
        player: PlayerId,
    },
    Evaded {
        player: PlayerId,
    },
    /// A player crossed the finish: `place` counts from 1, `time_ms` is the race time.
    RaceFinished {
        player: PlayerId,
        place: u8,
        time_ms: u32,
    },
    LapCompleted {
        player: PlayerId,
        lap: u8,
        time_ms: u32,
    },
}

/// An authoritative world advanced one tick at a time.
pub trait WorldSim {
    /// The tick the next [`WorldSim::step`] will produce.
    fn tick(&self) -> Tick;

    /// Put a player's car in the world. The player keeps `id` until [`WorldSim::remove_player`].
    fn add_player(&mut self, id: PlayerId, join: &PlayerJoin) -> Result<(), SimError>;

    /// Take a player's car out of the world.
    fn remove_player(&mut self, id: PlayerId);

    /// Advance one tick. A seated player missing from `inputs` drives with neutral input. Events are appended to `events`.
    fn step(&mut self, inputs: &SimInputs, events: &mut Vec<SimEvent>);

    /// Append every entity the server may replicate, in ascending id order.
    fn entities(&self, out: &mut Vec<EntityView>);

    /// The player's own-car block, or `None` when the player is not in the world.
    fn own_state(&self, id: PlayerId) -> Option<OwnState>;

    /// A hash of all state that affects later ticks. Two worlds that started equal and saw the same inputs have equal hashes.
    fn state_hash(&self) -> u64;
}
