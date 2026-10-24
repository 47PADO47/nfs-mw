//! Gameplay events (protocol.md section 7.6, channel `events`, server to client). Every event starts with an
//! [`EventHead`]; the bodies leave out the `tick` and `target_tick` fields the doc lists for some events, because the head
//! already carries them. Events are idempotent: a client ignores a `seq` it has applied.

use super::common::{Role, SessionPhase, ensure, is_slot};
use super::macros::{wire_enum, wire_struct};
use super::text::Name;
use super::wire::List;

/// Most entities named in one event list (start order, pursuit targets, result rows).
pub const MAX_EVENT_ENTITIES: usize = 32;

wire_struct! {
    /// The record header of every event.
    #[derive(Copy, Eq, Hash, Default)]
    pub struct EventHead {
        /// Per connection, +1 per event.
        pub seq: u32,
        /// The server tick when it happened.
        pub tick: u32,
        /// 0 when none, else the tick at which the client must apply it.
        pub target_tick: u32,
    }
}

wire_struct! {
    /// The race starts at `head.target_tick`; `order` is the grid by entity id.
    pub struct RaceStart {
        pub head: EventHead,
        pub order: List<u16, MAX_EVENT_ENTITIES>,
    }
}

wire_struct! {
    pub struct Checkpoint {
        pub head: EventHead,
        pub entity_id: u16,
        pub index: u16,
    }
}

wire_struct! {
    pub struct LapComplete {
        pub head: EventHead,
        pub entity_id: u16,
        pub lap: u8,
        pub time_ms: u32,
    }
}

wire_struct! {
    pub struct RaceFinish {
        pub head: EventHead,
        pub entity_id: u16,
        /// From 1.
        pub place: u8,
        pub time_ms: u32,
    }
}

wire_struct! {
    /// Last at the end of `lap` in a knockout: the racer is out and watches the rest.
    pub struct KnockedOut {
        pub head: EventHead,
        pub entity_id: u16,
        /// From 1.
        pub lap: u8,
    }
}

wire_struct! {
    /// A speed trap paid. Speeds are centimetres per second (the result record's unit).
    pub struct TrapScore {
        pub head: EventHead,
        pub entity_id: u16,
        pub trap: u8,
        /// The stored speed of this pass.
        pub speed_cm_s: u32,
        /// The racer's total points after it.
        pub total_cm_s: u32,
    }
}

wire_struct! {
    /// The tag moved. `from_entity` is `None` for the first holder.
    pub struct TagPassed {
        pub head: EventHead,
        pub from_entity: Option<u16>,
        pub to_entity: u16,
    }
}

wire_struct! {
    /// A reset added time to the racer's race time.
    pub struct Penalty {
        pub head: EventHead,
        pub entity_id: u16,
        pub ms: u32,
    }
}

/// Result row flag: the racer did not finish.
pub const RESULT_DNF: u8 = 1 << 0;
/// Result row flag: the racer was busted.
pub const RESULT_BUSTED: u8 = 1 << 1;

wire_struct! {
    /// One racer's line of a result record.
    #[derive(Copy, Eq, Hash)]
    pub struct ResultRow {
        pub slot: u8,
        pub entity_id: u16,
        pub place: u8,
        pub time_ms: u32,
        pub best_lap_ms: u32,
        /// [`RESULT_DNF`], [`RESULT_BUSTED`].
        pub flags: u8,
    }
    check(row) { ensure(is_slot(row.slot)) }
}

wire_struct! {
    /// The result record (also written to the replay).
    pub struct RaceResults {
        pub head: EventHead,
        pub rows: List<ResultRow, MAX_EVENT_ENTITIES>,
    }
}

wire_struct! {
    pub struct PursuitStart {
        pub head: EventHead,
        pub pursuit_id: u16,
        pub targets: List<u16, MAX_EVENT_ENTITIES>,
        pub heat: u8,
    }
}

wire_enum! {
    pub enum PursuitEndReason {
        Busted = 0,
        Evaded = 1,
        Abandoned = 2,
    }
}

wire_struct! {
    pub struct PursuitEnd {
        pub head: EventHead,
        pub pursuit_id: u16,
        pub reason: PursuitEndReason,
        pub entity_id: u16,
    }
}

wire_struct! {
    pub struct HeatChange {
        pub head: EventHead,
        pub entity_id: u16,
        pub level: u8,
    }
}

wire_struct! {
    pub struct Infraction {
        pub head: EventHead,
        pub entity_id: u16,
        /// The game's infraction id.
        pub kind: u8,
        pub fine: u32,
    }
}

wire_struct! {
    /// The busted meter moved a step: `fraction` 0..=255 is 0..1.
    pub struct BustedProgress {
        pub head: EventHead,
        pub entity_id: u16,
        pub fraction: u8,
    }
}

wire_enum! {
    pub enum RoadblockState {
        Forming = 0,
        Standing = 1,
        Dodged = 2,
        Destroyed = 3,
    }
}

wire_struct! {
    pub struct RoadblockEvent {
        pub head: EventHead,
        pub id: u16,
        pub state: RoadblockState,
        pub position: [f32; 3],
        pub yaw: f32,
    }
}

wire_struct! {
    pub struct SpikeEvent {
        pub head: EventHead,
        pub id: u16,
        pub state: RoadblockState,
        /// The wheel that hit the strip, `0..4`, if any.
        pub wheel: Option<u8>,
    }
    check(spike) { ensure(spike.wheel.is_none_or(|wheel| wheel < 4)) }
}

wire_struct! {
    pub struct Collision {
        pub head: EventHead,
        pub a: u16,
        /// The other entity, or `None` for the world.
        pub b: Option<u16>,
        pub impulse: f32,
        pub point: [f32; 3],
        pub normal: [f32; 3],
    }
}

wire_struct! {
    pub struct Damage {
        pub head: EventHead,
        pub entity_id: u16,
        pub zone: u8,
        pub severity: u8,
        pub glazed: bool,
    }
}

wire_struct! {
    pub struct Puncture {
        pub head: EventHead,
        pub entity_id: u16,
        /// `0..4`.
        pub wheel: u8,
    }
    check(puncture) { ensure(puncture.wheel < 4) }
}

wire_struct! {
    pub struct Repair {
        pub head: EventHead,
        pub entity_id: u16,
    }
}

wire_enum! {
    pub enum RespawnCause {
        /// The player asked (`ResetRequest`).
        Reset = 0,
        Busted = 1,
        /// The car left the world or fell through it.
        OutOfWorld = 2,
        Admin = 3,
        /// The player joined a running session.
        JoinInProgress = 4,
    }
}

wire_struct! {
    pub struct Respawn {
        pub head: EventHead,
        pub entity_id: u16,
        pub position: [f32; 3],
        pub yaw: f32,
        pub cause: RespawnCause,
    }
}

wire_struct! {
    /// The session moved to `phase`; `head.target_tick` is when it applies.
    pub struct PhaseChange {
        pub head: EventHead,
        pub phase: SessionPhase,
    }
}

wire_enum! {
    pub enum RemoveReason {
        Despawned = 0,
        OutOfRange = 1,
        /// Its player left.
        Left = 2,
        Destroyed = 3,
    }
}

wire_struct! {
    pub struct EntityRemoved {
        pub head: EventHead,
        pub entity_id: u16,
        pub reason: RemoveReason,
    }
}

wire_struct! {
    pub struct PlayerJoined {
        pub head: EventHead,
        pub slot: u8,
        pub player_id: [u8; 16],
        pub name: Name,
    }
    check(joined) { ensure(is_slot(joined.slot)) }
}

wire_enum! {
    pub enum LeaveReason {
        Quit = 0,
        Timeout = 1,
        Kicked = 2,
        Banned = 3,
    }
}

wire_struct! {
    pub struct PlayerLeft {
        pub head: EventHead,
        pub slot: u8,
        pub reason: LeaveReason,
    }
    check(left) { ensure(is_slot(left.slot)) }
}

wire_struct! {
    pub struct RoleChanged {
        pub head: EventHead,
        pub slot: u8,
        pub role: Role,
    }
    check(changed) { ensure(is_slot(changed.slot)) }
}

wire_struct! {
    /// The world clock: `time_of_day` in minutes after midnight, `rate` in game minutes per real second.
    pub struct ClockEvent {
        pub head: EventHead,
        pub time_of_day: u16,
        pub rate: f32,
    }
    check(clock) { ensure(clock.time_of_day < 24 * 60) }
}
