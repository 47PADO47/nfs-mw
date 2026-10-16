//! Generic event kinds and their payloads (ADR 0005 section 4; the catalogue is docs/modding/api.md#events).
//!
//! A `BbEvent` carries a kind, the frame number and an optional payload pointer. Each kind below names one payload
//! struct. Every payload starts with `size: u32`, the number of bytes the host filled in. A reader uses a payload only
//! when `size` covers the struct it knows; a payload that is missing or shorter is treated as an unknown event. Fields
//! are appended, never moved.
//!
//! Frame and tick events: `BB_EVENT_FRAME_START` is delivered before the scene updates in a frame, and
//! `BB_EVENT_SCENE_TICK` after the scene's update. Kind numbers are never reused; the snapshot in `event_tests.rs`
//! locks them.

/// `BbFrameStart`: a frame begins, before the scene updates.
pub const BB_EVENT_FRAME_START: u32 = 1;
/// `BbSceneTick`: the scene has updated for this frame.
pub const BB_EVENT_SCENE_TICK: u32 = 2;
/// `BbWorldLoaded`: a world finished loading.
pub const BB_EVENT_WORLD_LOADED: u32 = 3;
/// `BbWorldUnloading`: a world is about to be torn down.
pub const BB_EVENT_WORLD_UNLOADING: u32 = 4;
/// `BbVehicleSpawned`: a vehicle entered the world.
pub const BB_EVENT_VEHICLE_SPAWNED: u32 = 5;
/// `BbVehicleDespawned`: a vehicle left the world.
pub const BB_EVENT_VEHICLE_DESPAWNED: u32 = 6;
/// `BbVehicleCollision`: a vehicle hit something.
pub const BB_EVENT_VEHICLE_COLLISION: u32 = 7;
/// `BbHourChanged`: the whole hour of the day changed.
pub const BB_EVENT_HOUR_CHANGED: u32 = 8;
/// `BbMenuEvent`: a menu or a mod page opened.
pub const BB_EVENT_MENU_OPENED: u32 = 9;
/// `BbMenuEvent`: a menu or a mod page closed.
pub const BB_EVENT_MENU_CLOSED: u32 = 10;
/// `BbActionPressed`: an input action (a hotkey) was pressed this frame.
pub const BB_EVENT_ACTION_PRESSED: u32 = 11;
/// `BbAbilityActivated`: an ability was activated.
pub const BB_EVENT_ABILITY_ACTIVATED: u32 = 12;
/// `BbModUnloading`: a mod is about to be unloaded. Every loaded mod receives it, including the one that unloads.
pub const BB_EVENT_MOD_UNLOADING: u32 = 13;

/// `BbVehicleSpawned::role` and `BbVehicleDespawned::role`: the player's car.
pub const BB_ROLE_PLAYER: u32 = 0;
/// Traffic car.
pub const BB_ROLE_TRAFFIC: u32 = 1;
/// Police car.
pub const BB_ROLE_COP: u32 = 2;
/// Racing opponent.
pub const BB_ROLE_RACER: u32 = 3;

/// `BbVehicleCollision::other_kind`: another vehicle (`other_id` is its handle).
pub const BB_COLLIDE_VEHICLE: u32 = 0;
/// A prop (`other_id` is the host's prop id).
pub const BB_COLLIDE_PROP: u32 = 1;
/// A wall or the ground's static geometry (`other_id` is 0).
pub const BB_COLLIDE_WALL: u32 = 2;
/// Nothing that the host names (`other_id` is 0).
pub const BB_COLLIDE_NONE: u32 = 3;

/// Payload of `BB_EVENT_FRAME_START`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbFrameStart {
    /// Bytes the host filled in.
    pub size: u32,
    /// Real seconds since the previous frame.
    pub dt_real: f32,
}

/// Payload of `BB_EVENT_SCENE_TICK`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbSceneTick {
    /// Bytes the host filled in.
    pub size: u32,
    /// Simulated seconds this tick advanced the world by.
    pub dt_sim: f32,
    /// Current world time scale (1.0 is normal speed).
    pub time_scale: f32,
    /// Non-zero while the game is paused.
    pub paused: u8,
    /// Padding, zero.
    pub _pad: [u8; 3],
}

/// Payload of `BB_EVENT_WORLD_LOADED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbWorldLoaded {
    /// Bytes the host filled in.
    pub size: u32,
    /// Non-zero when streaming has finished and the world is ready to drive.
    pub streaming_ready: u8,
    /// Padding, zero.
    pub _pad: [u8; 3],
    /// World handle.
    pub world: u64,
}

/// Payload of `BB_EVENT_WORLD_UNLOADING`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbWorldUnloading {
    /// Bytes the host filled in.
    pub size: u32,
    /// Padding, zero.
    pub _pad: u32,
    /// World handle.
    pub world: u64,
}

/// Payload of `BB_EVENT_VEHICLE_SPAWNED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbVehicleSpawned {
    /// Bytes the host filled in.
    pub size: u32,
    /// `BB_ROLE_*`.
    pub role: u32,
    /// Vehicle handle.
    pub vehicle: u64,
    /// The car's id in the host's car registry.
    pub car_id: u64,
}

/// Payload of `BB_EVENT_VEHICLE_DESPAWNED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbVehicleDespawned {
    /// Bytes the host filled in.
    pub size: u32,
    /// `BB_ROLE_*`.
    pub role: u32,
    /// Vehicle handle. It is stale once this event has been delivered.
    pub vehicle: u64,
}

/// Payload of `BB_EVENT_VEHICLE_COLLISION`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbVehicleCollision {
    /// Bytes the host filled in.
    pub size: u32,
    /// `BB_COLLIDE_*`: what the vehicle hit.
    pub other_kind: u32,
    /// The vehicle that collided.
    pub vehicle: u64,
    /// Handle or id of what it hit, as `other_kind` says (0 when there is none).
    pub other_id: u64,
    /// World point of contact, render space (x forward, y left, z up), metres.
    pub point: [f32; 3],
    /// Impulse of the impact, in the host's units.
    pub impulse: f32,
}

/// Payload of `BB_EVENT_HOUR_CHANGED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbHourChanged {
    /// Bytes the host filled in.
    pub size: u32,
    /// The new whole hour, 0 to 23.
    pub hour: u32,
}

/// Payload of `BB_EVENT_MENU_OPENED` and `BB_EVENT_MENU_CLOSED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbMenuEvent {
    /// Bytes the host filled in.
    pub size: u32,
    /// Padding, zero.
    pub _pad: u32,
    /// Menu handle: a host screen or a mod page.
    pub menu: u64,
}

/// Payload of `BB_EVENT_ACTION_PRESSED`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BbActionPressed {
    /// Bytes the host filled in.
    pub size: u32,
    /// Padding, zero.
    pub _pad: u32,
    /// Action handle. A mod action has one; a vanilla action is named only by `name`, and then this is 0.
    pub action: u64,
    /// The action's name, valid for the call.
    pub name: crate::abi::BbStr,
}

/// Payload of `BB_EVENT_ABILITY_ACTIVATED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbAbilityActivated {
    /// Bytes the host filled in.
    pub size: u32,
    /// Padding, zero.
    pub _pad: u32,
    /// Ability handle.
    pub ability: u64,
}

/// Payload of `BB_EVENT_MOD_UNLOADING`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BbModUnloading {
    /// Bytes the host filled in.
    pub size: u32,
    /// Padding, zero.
    pub _pad: u32,
    /// Id of the mod being unloaded, valid for the call.
    pub mod_id: crate::abi::BbStr,
}
