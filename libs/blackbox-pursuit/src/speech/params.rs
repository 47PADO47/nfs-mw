//! What a request says about the situation: the typed values the radio's sentences are worded from. Every field is
//! optional (a line sets the ones its sentence function takes); the game turns them into the parameters of its speech
//! interface. Opaque ids (a battalion, a car's type and colour, a road) are the game's own numbers: the machine only
//! passes them through.

/// The kind of offence the opening line names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PursuitType {
    GenericSpeeder,
    PossibleWanted,
    HitAndRun,
    Reckless,
    UnitRammed,
}

/// One suspect or several (AI racers within range).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suspects {
    One,
    Multiple,
}

/// A unit's battalion, as the game numbers them (what a cop calls himself).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Battalion(pub u16);

/// Another unit: its battalion and call sign number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unit {
    pub battalion: Battalion,
    pub call_sign: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intensity {
    Normal,
    High,
}

/// How long the contact was lost before the pursuit resumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LostFor {
    Recently,
    ForAWhile,
}

/// The player's car as the game describes it: its type and colour are the game's class numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Car {
    pub kind: u32,
    pub colour: u32,
}

/// Which unit of speed a speed is spoken in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Measurement {
    Generic,
    Imperial,
    Metric,
}

/// The compass side the suspect is heading to, from the dominant axis of its velocity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Heading along the world's negative Z axis (the original's "northbound").
    North,
    /// Along positive Z ("southbound").
    South,
    /// Along positive X ("eastbound").
    East,
    /// Along negative X ("westbound").
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encounter {
    First,
    Subsequent,
}

/// What a request for backup asks for (also what dispatch answers with).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backup {
    NoUnits,
    OneUnit,
    AllUnits,
    AirSupport,
    Rhino,
    GroundUnits,
    SuperPursuit,
}

/// Dispatch's estimate of when backup arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eta {
    Secs15,
    Secs30,
    Min1,
    Min1Secs30,
    Min2,
    Over2Min,
}

/// What a lone cop says it will do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfStrategy {
    Pit,
    SelfPit,
    SideRam,
    Ram,
    RollingRoadblock,
    Herding,
}

/// The formation a group of cops announces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollingStrategy {
    Pit,
    Box,
    RollingRoadblock,
    Herding,
}

/// Where a cop is told to take position, from his offset to the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Right,
    Left,
    Ahead,
    Behind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldObject {
    Generic,
    Guardrail,
    Train,
    Semi,
    GasStation,
    SpikeBelt,
    /// The first, mild collision of a low-heat chase.
    LowHeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    One,
    Multiple,
}

/// Why a cop leaves the chase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bailout {
    LowIntensity,
    HeavyTraffic,
    RoadConditions,
    Damage,
    HighIntensity,
}

/// What a call for an emergency vehicle is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ev {
    MultipleUnitsDown,
    GasStationFire,
    HeliDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jurisdiction {
    State,
    Federal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadblockKind {
    Generic,
    Spikes,
    HeliDispatchOnly,
    MultipleDispatchOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engage {
    Roadblock,
    Spikes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpikePosition {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeliStrategy {
    /// Offset follow (the helicopter's ordinary self strategy).
    Follow,
    Roadblock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeliLost {
    InTunnel,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeliBailout {
    FlightConditions,
    FuelLow,
    DamageSustained,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hazard {
    ApproachingTunnel,
}

/// The parameters of one request. A field left `None` is not set (the line's wording is open for it).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Params {
    pub pursuit_type: Option<PursuitType>,
    pub suspects: Option<Suspects>,
    /// The speaker's own battalion and call sign number (1 up).
    pub battalion: Option<Battalion>,
    pub call_sign: Option<u8>,
    /// The unit the line is about (a call to position, a dispatch reply to a unit).
    pub subject: Option<Unit>,
    pub intensity: Option<Intensity>,
    /// A yes or a no (a reply to a request).
    pub yes: Option<bool>,
    pub lost_for: Option<LostFor>,
    pub car: Option<Car>,
    /// A speed in the unit the player's speedometer shows.
    pub speed: Option<f32>,
    pub measurement: Option<Measurement>,
    pub direction: Option<Direction>,
    /// The speech id of the road the suspect is on (the game turns it into a place name).
    pub road: Option<u16>,
    pub encounter: Option<Encounter>,
    pub backup: Option<Backup>,
    pub eta: Option<Eta>,
    pub strategy: Option<SelfStrategy>,
    pub rolling: Option<RollingStrategy>,
    pub position: Option<Position>,
    /// `Some(true)` for a new strategy, `Some(false)` for the same one again.
    pub new_strategy: Option<bool>,
    pub world_object: Option<WorldObject>,
    pub units: Option<Units>,
    pub bailout: Option<Bailout>,
    /// The line is about another unit (not the speaker).
    pub about_other: Option<bool>,
    pub ev: Option<Ev>,
    /// An off-road place (the speech map's id) and whether it was commented on before.
    pub offroad: Option<(u8, bool)>,
    /// The heat level reached (2 to 5).
    pub heat_level: Option<u8>,
    pub jurisdiction: Option<Jurisdiction>,
    pub roadblock: Option<RoadblockKind>,
    pub engage: Option<Engage>,
    pub spike_position: Option<SpikePosition>,
    pub heli_strategy: Option<HeliStrategy>,
    pub heli_lost: Option<HeliLost>,
    pub heli_bailout: Option<HeliBailout>,
    pub hazard: Option<Hazard>,
    /// The address group of dispatch's escalation: that of the area this battalion patrols.
    pub address_of: Option<Battalion>,
    /// The 911 report names the area from the place, or says nothing specific (one time in three).
    pub generic_address: Option<bool>,
}
