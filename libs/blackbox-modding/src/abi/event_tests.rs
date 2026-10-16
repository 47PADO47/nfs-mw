//! Snapshot of the event kinds and the payload layouts (ADR 0005 section 9). Changing a number here is an ABI change:
//! edit it on purpose, in the same commit, and follow the rules in `mod.rs`.

use core::mem::{align_of, offset_of, size_of};

use crate::abi::*;

#[test]
fn generic_event_kinds_are_locked() {
    assert_eq!(
        [
            BB_EVENT_FRAME_START,
            BB_EVENT_SCENE_TICK,
            BB_EVENT_WORLD_LOADED,
            BB_EVENT_WORLD_UNLOADING,
            BB_EVENT_VEHICLE_SPAWNED,
            BB_EVENT_VEHICLE_DESPAWNED,
            BB_EVENT_VEHICLE_COLLISION,
            BB_EVENT_HOUR_CHANGED,
            BB_EVENT_MENU_OPENED,
            BB_EVENT_MENU_CLOSED,
            BB_EVENT_ACTION_PRESSED,
            BB_EVENT_ABILITY_ACTIVATED,
            BB_EVENT_MOD_UNLOADING,
        ],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]
    );
}

#[test]
fn roles_and_collision_kinds_are_locked() {
    assert_eq!([BB_ROLE_PLAYER, BB_ROLE_TRAFFIC, BB_ROLE_COP, BB_ROLE_RACER], [0, 1, 2, 3]);
    assert_eq!([BB_COLLIDE_VEHICLE, BB_COLLIDE_PROP, BB_COLLIDE_WALL, BB_COLLIDE_NONE], [0, 1, 2, 3]);
}

#[test]
fn handles_pack_index_and_generation() {
    assert_eq!(BB_NULL_HANDLE, 0);
    let handle = make_handle(7, 3);
    assert_eq!((handle_index(handle), handle_generation(handle)), (7, 3));
    assert_eq!(handle, (3u64 << 32) | 7);
}

#[cfg(target_pointer_width = "64")]
mod layout64 {
    use super::*;

    #[test]
    fn frame_start() {
        assert_eq!(size_of::<BbFrameStart>(), 8);
        assert_eq!(align_of::<BbFrameStart>(), 4);
        assert_eq!(offset_of!(BbFrameStart, dt_real), 4);
    }

    #[test]
    fn scene_tick() {
        assert_eq!(size_of::<BbSceneTick>(), 16);
        assert_eq!(offset_of!(BbSceneTick, dt_sim), 4);
        assert_eq!(offset_of!(BbSceneTick, time_scale), 8);
        assert_eq!(offset_of!(BbSceneTick, paused), 12);
    }

    #[test]
    fn world_events() {
        assert_eq!(size_of::<BbWorldLoaded>(), 16);
        assert_eq!(offset_of!(BbWorldLoaded, streaming_ready), 4);
        assert_eq!(offset_of!(BbWorldLoaded, world), 8);
        assert_eq!(size_of::<BbWorldUnloading>(), 16);
        assert_eq!(offset_of!(BbWorldUnloading, world), 8);
    }

    #[test]
    fn vehicle_events() {
        assert_eq!(size_of::<BbVehicleSpawned>(), 24);
        assert_eq!(offset_of!(BbVehicleSpawned, role), 4);
        assert_eq!(offset_of!(BbVehicleSpawned, vehicle), 8);
        assert_eq!(offset_of!(BbVehicleSpawned, car_id), 16);
        assert_eq!(size_of::<BbVehicleDespawned>(), 16);
        assert_eq!(offset_of!(BbVehicleDespawned, vehicle), 8);
    }

    #[test]
    fn vehicle_collision() {
        assert_eq!(size_of::<BbVehicleCollision>(), 40);
        assert_eq!(align_of::<BbVehicleCollision>(), 8);
        assert_eq!(offset_of!(BbVehicleCollision, other_kind), 4);
        assert_eq!(offset_of!(BbVehicleCollision, vehicle), 8);
        assert_eq!(offset_of!(BbVehicleCollision, other_id), 16);
        assert_eq!(offset_of!(BbVehicleCollision, point), 24);
        assert_eq!(offset_of!(BbVehicleCollision, impulse), 36);
    }

    #[test]
    fn hour_and_menu_events() {
        assert_eq!(size_of::<BbHourChanged>(), 8);
        assert_eq!(offset_of!(BbHourChanged, hour), 4);
        assert_eq!(size_of::<BbMenuEvent>(), 16);
        assert_eq!(offset_of!(BbMenuEvent, menu), 8);
    }

    #[test]
    fn action_ability_and_unload_events() {
        assert_eq!(size_of::<BbActionPressed>(), 32);
        assert_eq!(offset_of!(BbActionPressed, action), 8);
        assert_eq!(offset_of!(BbActionPressed, name), 16);
        assert_eq!(size_of::<BbAbilityActivated>(), 16);
        assert_eq!(offset_of!(BbAbilityActivated, ability), 8);
        assert_eq!(size_of::<BbModUnloading>(), 24);
        assert_eq!(offset_of!(BbModUnloading, mod_id), 8);
    }
}
