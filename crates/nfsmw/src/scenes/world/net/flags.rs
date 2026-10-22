//! The flag bits of the entity views the host exports ([`nfsmw_sim::EntityView::flags`]) and of the own-car block
//! ([`nfsmw_sim::OwnState::flags`]). Game-defined [ours]; the wire keeps the low bits each kind has room for
//! (`nfsmw_net::schema::layout`), and every bit here is inside the smallest of them (8).

/// A car's brake lamps are lit.
pub const FLAG_BRAKING: u32 = 1 << 0;
/// A police car's light bar flashes.
pub const FLAG_LIGHTS: u32 = 1 << 1;
/// The car is wrecked.
pub const FLAG_WRECKED: u32 = 1 << 2;
/// A player car's headlights are on.
pub const FLAG_HEADLIGHTS: u32 = 1 << 3;
/// A [`nfsmw_sim::EntityKind::Roadblock`] entity that is a car of the roadblock (its `type_id` is the car's model).
pub const FLAG_ROADBLOCK_CAR: u32 = 1 << 4;
/// A [`nfsmw_sim::EntityKind::Roadblock`] entity that is a spike strip (its `type_id` is [`SPIKE_STRIP_TYPE`]).
pub const FLAG_SPIKE_STRIP: u32 = 1 << 5;

/// The `type_id` of a spike strip.
pub const SPIKE_STRIP_TYPE: u16 = 0xFFFE;

/// Own-car flag: the speedbreaker is on.
pub const OWN_SPEEDBREAKER: u8 = 1 << 0;
/// Own-car flag: a gear change is in progress.
pub const OWN_SHIFTING: u8 = 1 << 1;
/// Own-car flag: the gearbox wants the next gear (the shift light).
pub const OWN_SHIFT_LIGHT: u8 = 1 << 2;
/// Own-car flag: the car waits for its road (it joined, respawned or was reset, and is not placed yet).
pub const OWN_WAITING: u8 = 1 << 3;
/// Own-car flag: the player is out of play for now (a race seat or a cutscene holds the car).
pub const OWN_PARKED: u8 = 1 << 4;
