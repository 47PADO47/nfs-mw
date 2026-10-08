//! The numbers of the original's sound objects and controllers, and the output slots each reads. Spec:
//! `docs/specs/car-sound-mixer.md` §1 to §3.

use blackbox_mixmap::{InputKey, ObjectRef};

/// The main state (volume sliders) and the player's car state.
pub const MAIN: u8 = 0;
pub const PLAYER: u8 = 2;

/// Sound objects of the player's car.
pub mod object {
    pub const ENGINE_SINGLE: u8 = 1;
    pub const ENGINE_DUAL: u8 = 2;
    pub const SHIFT: u8 = 3;
    pub const TURBO: u8 = 4;
    pub const NITROUS: u8 = 5;
    pub const SPARKS: u8 = 6;
    pub const SKIDS: u8 = 7;
    pub const ROAD: u8 = 8;
    pub const WIND: u8 = 9;
    pub const BOTTOM_OUT: u8 = 13;
}

/// Sound controllers of the player's car.
pub mod controller {
    pub const PHYSICS: u8 = 0;
    pub const ENGINE: u8 = 4;
    pub const HYBRID: u8 = 5;
    pub const CAR_POSITION: u8 = 7;
    pub const REAR_POSITION: u8 = 8;
    pub const OBJECT_POSITION: u8 = 10;
    pub const RIGHT_WHEEL: u8 = 11;
    pub const LEFT_WHEEL: u8 = 12;
    pub const LEFT_WIND: u8 = 13;
    pub const RIGHT_WIND: u8 = 14;
}

/// Output slots (indices into an object's output block).
pub mod slot {
    pub const ENGINE_GINSU: usize = 2;
    pub const ENGINE_PITCH: usize = 4;
    pub const CLUNK_UP: usize = 1;
    pub const CLUNK_DOWN: usize = 2;
    pub const SWEET_ENGAGE: usize = 3;
    pub const SWEET_DISENGAGE: usize = 4;
    pub const SWEET_ACCELERATE: usize = 5;
    pub const SWEET_ENGINE_OFF: usize = 6;
    pub const WHINE: usize = 8;
    pub const BRAKE_MASH: usize = 10;
    pub const TURBO_SPOOL: usize = 1;
    pub const TURBO_BLOWOFF_FIRST: usize = 2;
    pub const TURBO_BLOWOFF_OTHER: usize = 3;
    pub const NITROUS: usize = 1;
    pub const PURGE: usize = 2;
    pub const NITROUS_PITCH: usize = 3;
    pub const SKID_FORWARD: usize = 5;
    pub const SKID_BACK: usize = 6;
    pub const SKID_SIDE: usize = 7;
    pub const SKID_PITCH: usize = 8;
    pub const WIND_LEFT: usize = 2;
    pub const WIND_RIGHT: usize = 3;
    pub const WIND_PITCH: usize = 5;
    pub const LANDING: usize = 1;

    /// The road noise slot of a loop value (0 gravel, 1 sidewalk, 2 cobblestone, 3 deep water, 4 wet road,
    /// 5 and 6 asphalt, 7 metal, 8 stitch loop).
    pub const ROAD: [usize; 9] = [5, 6, 7, 8, 9, 10, 10, 11, 12];
}

pub fn player_object(object: u8) -> ObjectRef {
    ObjectRef { state: PLAYER, instance: 0, object }
}

pub fn player_controller(controller: u8, index: u8) -> InputKey {
    InputKey::controller(PLAYER, 0, controller, index)
}

pub fn player_object_input(object: u8, index: u8) -> InputKey {
    InputKey::object(PLAYER, 0, object, index)
}
