//! The typed event a mod receives in `Mod::event` (ADR 0005 section 2). The SDK decodes it from the `BbEvent` the host
//! sends (see `ffi::event`); an event the SDK cannot describe arrives as [`Event::Unknown`].

use crate::abi::{
    BB_COLLIDE_NONE, BB_COLLIDE_PROP, BB_COLLIDE_VEHICLE, BB_COLLIDE_WALL, BB_EVENT_ABILITY_ACTIVATED,
    BB_EVENT_ACTION_PRESSED, BB_EVENT_FRAME_START, BB_EVENT_HOUR_CHANGED, BB_EVENT_INTERIOR_ENTERED,
    BB_EVENT_INTERIOR_ENTERING, BB_EVENT_INTERIOR_EXITED, BB_EVENT_INTERIOR_EXITING, BB_EVENT_MENU_CLOSED,
    BB_EVENT_MENU_OPENED, BB_EVENT_MOD_UNLOADING, BB_EVENT_SCENE_TICK, BB_EVENT_VEHICLE_COLLISION,
    BB_EVENT_VEHICLE_DESPAWNED, BB_EVENT_VEHICLE_SPAWNED, BB_EVENT_WORLD_LOADED, BB_EVENT_WORLD_UNLOADING, BB_ROLE_COP,
    BB_ROLE_PLAYER, BB_ROLE_RACER, BB_ROLE_TRAFFIC,
};

use super::{AbilityId, ActionId, MenuId, VehicleId, WorldId};

/// What a vehicle is, as the host names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The player's car.
    Player,
    /// A traffic car.
    Traffic,
    /// A police car.
    Cop,
    /// A racing opponent.
    Racer,
}

impl Role {
    /// The role a `BB_ROLE_*` value names. `None` for a value this SDK does not know.
    pub(crate) fn from_raw(raw: u32) -> Option<Role> {
        match raw {
            BB_ROLE_PLAYER => Some(Role::Player),
            BB_ROLE_TRAFFIC => Some(Role::Traffic),
            BB_ROLE_COP => Some(Role::Cop),
            BB_ROLE_RACER => Some(Role::Racer),
            _ => None,
        }
    }
}

/// What a vehicle hit, as the host names it. The matching id is `Event::VehicleCollision::other_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Collider {
    /// Another vehicle.
    Vehicle,
    /// A prop.
    Prop,
    /// A wall or static geometry.
    Wall,
    /// Nothing the host names.
    None,
}

impl Collider {
    /// The collider a `BB_COLLIDE_*` value names. `None` for a value this SDK does not know.
    pub(crate) fn from_raw(raw: u32) -> Option<Collider> {
        match raw {
            BB_COLLIDE_VEHICLE => Some(Collider::Vehicle),
            BB_COLLIDE_PROP => Some(Collider::Prop),
            BB_COLLIDE_WALL => Some(Collider::Wall),
            BB_COLLIDE_NONE => Some(Collider::None),
            _ => None,
        }
    }
}

/// One event delivered to `Mod::event`. Every variant carries the frame number of its delivery.
///
/// Names (`ActionPressed`, `ModUnloading`) are borrowed for the call only. Copy what you keep.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Event<'a> {
    /// A frame begins, before the scene updates.
    FrameStart {
        /// Frame number.
        frame: u64,
        /// Real seconds since the previous frame.
        dt_real: f32,
    },
    /// The scene has updated.
    SceneTick {
        /// Frame number.
        frame: u64,
        /// Simulated seconds this tick advanced the world by.
        dt_sim: f32,
        /// Current world time scale.
        time_scale: f32,
        /// Whether the game is paused.
        paused: bool,
    },
    /// A world finished loading.
    WorldLoaded {
        /// Frame number.
        frame: u64,
        /// The world.
        world: WorldId,
        /// Whether streaming has finished and the world can be driven.
        streaming_ready: bool,
    },
    /// A world is about to be torn down.
    WorldUnloading {
        /// Frame number.
        frame: u64,
        /// The world.
        world: WorldId,
    },
    /// A vehicle entered the world.
    VehicleSpawned {
        /// Frame number.
        frame: u64,
        /// The vehicle.
        vehicle: VehicleId,
        /// Its role.
        role: Role,
        /// Its car id in the host's car registry.
        car_id: u64,
    },
    /// A vehicle left the world. Its handle is stale from here on.
    VehicleDespawned {
        /// Frame number.
        frame: u64,
        /// The vehicle.
        vehicle: VehicleId,
        /// Its role.
        role: Role,
    },
    /// A vehicle hit something.
    VehicleCollision {
        /// Frame number.
        frame: u64,
        /// The vehicle.
        vehicle: VehicleId,
        /// What it hit.
        other: Collider,
        /// The id of what it hit (0 when there is none); for a vehicle, its raw handle.
        other_id: u64,
        /// Impulse of the impact.
        impulse: f32,
        /// Point of contact, render space (x forward, y left, z up), metres.
        point: [f32; 3],
    },
    /// The whole hour of the day changed.
    HourChanged {
        /// Frame number.
        frame: u64,
        /// The new hour, 0 to 23.
        hour: u32,
    },
    /// A menu or mod page opened.
    MenuOpened {
        /// Frame number.
        frame: u64,
        /// The menu.
        menu: MenuId,
    },
    /// A menu or mod page closed.
    MenuClosed {
        /// Frame number.
        frame: u64,
        /// The menu.
        menu: MenuId,
    },
    /// An input action (a hotkey) was pressed this frame.
    ActionPressed {
        /// Frame number.
        frame: u64,
        /// The action; a vanilla action has no handle, see `name`.
        action: ActionId,
        /// The action's name.
        name: &'a str,
    },
    /// An ability was activated.
    AbilityActivated {
        /// Frame number.
        frame: u64,
        /// The ability.
        ability: AbilityId,
    },
    /// A mod is about to be unloaded. Every loaded mod receives this.
    ModUnloading {
        /// Frame number.
        frame: u64,
        /// The id of the mod being unloaded.
        mod_id: &'a str,
    },
    /// The player has started going into a mod interior. `position` is where the car stands now.
    InteriorEntering {
        /// Frame number.
        frame: u64,
        /// The interior's key (its manifest's file name).
        interior: &'a str,
        /// Render space (x forward, y left, z up), metres.
        position: [f32; 3],
    },
    /// The player is inside a mod interior, at its spawn.
    InteriorEntered {
        /// Frame number.
        frame: u64,
        /// The interior's key.
        interior: &'a str,
        /// The spawn inside, render space, metres.
        position: [f32; 3],
    },
    /// The player has started leaving a mod interior, from its spawn.
    InteriorExiting {
        /// Frame number.
        frame: u64,
        /// The interior's key.
        interior: &'a str,
        /// The spawn inside, render space, metres.
        position: [f32; 3],
    },
    /// The player is back outside. An entry cancelled before the player got in ends here too, straight after
    /// `InteriorEntering`.
    InteriorExited {
        /// Frame number.
        frame: u64,
        /// The interior's key.
        interior: &'a str,
        /// Where the car returns to outside, render space, metres.
        position: [f32; 3],
    },
    /// An event this SDK does not describe: an unknown kind, or a payload the host did not fill in far enough. The
    /// payload's bytes come with it, so a mod can decode a kind of its own (an `nfsmw.*` event, for example).
    Unknown {
        /// The kind as the host sent it.
        kind: u32,
        /// Frame number.
        frame: u64,
        /// The payload as the host filled it in: the bytes its size word declares, starting with that word. Empty when
        /// the event carries no payload. Valid for the callback only; copy what you keep.
        payload: &'a [u8],
    },
}

impl Event<'_> {
    /// The kind number of this event (`BB_EVENT_*`). For [`Event::Unknown`], the kind the host sent.
    pub fn kind(&self) -> u32 {
        match self {
            Event::FrameStart { .. } => BB_EVENT_FRAME_START,
            Event::SceneTick { .. } => BB_EVENT_SCENE_TICK,
            Event::WorldLoaded { .. } => BB_EVENT_WORLD_LOADED,
            Event::WorldUnloading { .. } => BB_EVENT_WORLD_UNLOADING,
            Event::VehicleSpawned { .. } => BB_EVENT_VEHICLE_SPAWNED,
            Event::VehicleDespawned { .. } => BB_EVENT_VEHICLE_DESPAWNED,
            Event::VehicleCollision { .. } => BB_EVENT_VEHICLE_COLLISION,
            Event::HourChanged { .. } => BB_EVENT_HOUR_CHANGED,
            Event::MenuOpened { .. } => BB_EVENT_MENU_OPENED,
            Event::MenuClosed { .. } => BB_EVENT_MENU_CLOSED,
            Event::ActionPressed { .. } => BB_EVENT_ACTION_PRESSED,
            Event::AbilityActivated { .. } => BB_EVENT_ABILITY_ACTIVATED,
            Event::ModUnloading { .. } => BB_EVENT_MOD_UNLOADING,
            Event::InteriorEntering { .. } => BB_EVENT_INTERIOR_ENTERING,
            Event::InteriorEntered { .. } => BB_EVENT_INTERIOR_ENTERED,
            Event::InteriorExiting { .. } => BB_EVENT_INTERIOR_EXITING,
            Event::InteriorExited { .. } => BB_EVENT_INTERIOR_EXITED,
            Event::Unknown { kind, .. } => *kind,
        }
    }

    /// The frame number at delivery.
    pub fn frame(&self) -> u64 {
        match self {
            Event::FrameStart { frame, .. }
            | Event::SceneTick { frame, .. }
            | Event::WorldLoaded { frame, .. }
            | Event::WorldUnloading { frame, .. }
            | Event::VehicleSpawned { frame, .. }
            | Event::VehicleDespawned { frame, .. }
            | Event::VehicleCollision { frame, .. }
            | Event::HourChanged { frame, .. }
            | Event::MenuOpened { frame, .. }
            | Event::MenuClosed { frame, .. }
            | Event::ActionPressed { frame, .. }
            | Event::AbilityActivated { frame, .. }
            | Event::ModUnloading { frame, .. }
            | Event::InteriorEntering { frame, .. }
            | Event::InteriorEntered { frame, .. }
            | Event::InteriorExiting { frame, .. }
            | Event::InteriorExited { frame, .. }
            | Event::Unknown { frame, .. } => *frame,
        }
    }
}
