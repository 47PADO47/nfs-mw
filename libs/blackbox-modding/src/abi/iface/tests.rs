//! Layout snapshot of the interface tables (ADR 0005 section 9). Changing a number here is an ABI change.

#![cfg(target_pointer_width = "64")]

use core::mem::{align_of, offset_of, size_of};

use crate::abi::*;

#[test]
fn ids_and_versions_are_locked() {
    assert_eq!(IFACE_COMMANDS, "bb.commands");
    assert_eq!(IFACE_INPUT, "bb.input");
    assert_eq!(IFACE_TIME, "bb.time");
    assert_eq!(IFACE_MAP, "bb.map");
    assert_eq!(IFACE_MENUS, "bb.menus");
    assert_eq!(
        [IFACE_COMMANDS_VERSION, IFACE_INPUT_VERSION, IFACE_TIME_VERSION, IFACE_MAP_VERSION, IFACE_MENUS_VERSION],
        [(1, 0), (1, 0), (1, 0), (1, 0), (1, 0)]
    );
    assert_eq!(
        [IFACE_VEHICLES, IFACE_HUD, IFACE_CAMERA, IFACE_AUDIO, IFACE_WORLD],
        ["bb.vehicles", "bb.hud", "bb.camera", "bb.audio", "bb.world"]
    );
    assert_eq!(
        [IFACE_VEHICLES_VERSION, IFACE_HUD_VERSION, IFACE_CAMERA_VERSION, IFACE_AUDIO_VERSION, IFACE_WORLD_VERSION],
        [(1, 2), (1, 0), (1, 1), (1, 0), (1, 0)]
    );
    assert_eq!(IFACE_UI, "bb.ui");
    assert_eq!(IFACE_UI_VERSION, (1, 1));
}

#[test]
fn ui_table_and_prim() {
    assert_eq!(size_of::<BbUiPrim>(), 96);
    assert_eq!(offset_of!(BbUiPrim, kind), 4);
    assert_eq!(offset_of!(BbUiPrim, x), 8);
    assert_eq!(offset_of!(BbUiPrim, colour), 24);
    assert_eq!(offset_of!(BbUiPrim, justify), 28);
    assert_eq!(offset_of!(BbUiPrim, uv), 32);
    assert_eq!(offset_of!(BbUiPrim, font), 48);
    assert_eq!(offset_of!(BbUiPrim, texture), 64);
    assert_eq!(offset_of!(BbUiPrim, text), 80);
    assert_eq!(size_of::<BbUi>(), 48);
    assert_eq!(offset_of!(BbUi, ctx), 8);
    assert_eq!(offset_of!(BbUi, register_theme), 16);
    assert_eq!(offset_of!(BbUi, set_slot), 24);
    assert_eq!(offset_of!(BbUi, unregister_theme), 32);
    assert_eq!(offset_of!(BbUi, set_tokens), 40);
    assert_eq!([BB_UI_PANEL, BB_UI_TEXT, BB_UI_IMAGE], [0, 1, 2]);
}

#[test]
fn vehicles_table_and_state() {
    assert_eq!(size_of::<BbVec3>(), 12);
    assert_eq!(size_of::<BbVehicles>(), 64);
    assert_eq!(offset_of!(BbVehicles, ctx), 8);
    assert_eq!(offset_of!(BbVehicles, state), 16);
    assert_eq!(offset_of!(BbVehicles, teleport), 24);
    assert_eq!(offset_of!(BbVehicles, repair), 32);
    assert_eq!(offset_of!(BbVehicles, spawn_traffic), 40);
    assert_eq!(offset_of!(BbVehicles, add_nos), 48);
    assert_eq!(offset_of!(BbVehicles, set_drift_assist), 56);
    assert_eq!(size_of::<BbVehicleState>(), 68);
    assert_eq!(offset_of!(BbVehicleState, gear), 4);
    assert_eq!(offset_of!(BbVehicleState, position), 8);
    assert_eq!(offset_of!(BbVehicleState, velocity), 20);
    assert_eq!(offset_of!(BbVehicleState, heading), 32);
    assert_eq!(offset_of!(BbVehicleState, speed), 36);
    assert_eq!(offset_of!(BbVehicleState, rpm), 40);
    assert_eq!(offset_of!(BbVehicleState, nos), 44);
    assert_eq!(offset_of!(BbVehicleState, damage), 48);
    // 1.1 appends these four; the 1.0 fields above stay where they were.
    assert_eq!(offset_of!(BbVehicleState, slip_angle), 52);
    assert_eq!(offset_of!(BbVehicleState, steering), 56);
    assert_eq!(offset_of!(BbVehicleState, impact), 60);
    assert_eq!(offset_of!(BbVehicleState, on_ground), 64);
}

#[test]
fn hud_and_message_styles() {
    assert_eq!(size_of::<BbHud>(), 32);
    assert_eq!(offset_of!(BbHud, message), 16);
    assert_eq!(offset_of!(BbHud, hide), 24);
    assert_eq!([BB_HUD_INFO, BB_HUD_WARNING, BB_HUD_BIG], [0, 1, 2]);
}

#[test]
fn camera_table_state_and_modes() {
    assert_eq!(size_of::<BbCamera>(), 56);
    assert_eq!(offset_of!(BbCamera, get), 16);
    assert_eq!(offset_of!(BbCamera, set_mode), 24);
    assert_eq!(offset_of!(BbCamera, register_view), 32);
    assert_eq!(offset_of!(BbCamera, unregister_view), 40);
    assert_eq!(offset_of!(BbCamera, select_view), 48);
    assert_eq!(size_of::<BbCameraInput>(), 120);
    assert_eq!(offset_of!(BbCameraInput, dt), 8);
    assert_eq!(offset_of!(BbCameraInput, position), 40);
    assert_eq!(offset_of!(BbCameraInput, angular_velocity), 88);
    assert_eq!(offset_of!(BbCameraInput, car), 104);
    assert_eq!(size_of::<BbCameraPose>(), 52);
    assert_eq!(offset_of!(BbCameraPose, position), 8);
    assert_eq!(offset_of!(BbCameraPose, fov), 44);
    assert_eq!(size_of::<BbCameraView>(), 64);
    assert_eq!(offset_of!(BbCameraView, user), 40);
    assert_eq!(offset_of!(BbCameraView, update), 56);
    assert_eq!([BB_CAMERA_EXTRA, BB_CAMERA_INPUT_RESET, BB_CAMERA_POSE_HIDE_CAR], [100, 1, 1]);
    assert_eq!(size_of::<BbCameraState>(), 36);
    assert_eq!(offset_of!(BbCameraState, mode), 4);
    assert_eq!(offset_of!(BbCameraState, position), 8);
    assert_eq!(offset_of!(BbCameraState, forward), 20);
    assert_eq!(offset_of!(BbCameraState, fov), 32);
    assert_eq!(
        [
            BB_CAMERA_CHASE,
            BB_CAMERA_DYNAMIC_FOV,
            BB_CAMERA_BUMPER,
            BB_CAMERA_HOOD,
            BB_CAMERA_CLOSE,
            BB_CAMERA_FAR,
            BB_CAMERA_SUPER_FAR,
            BB_CAMERA_DRIFT,
            BB_CAMERA_PURSUIT
        ],
        [0, 1, 2, 3, 4, 5, 6, 7, 8]
    );
}

#[test]
fn audio_and_world_tables() {
    assert_eq!(size_of::<BbAudio>(), 24);
    assert_eq!(offset_of!(BbAudio, play), 16);
    assert_eq!(size_of::<BbWorld>(), 48);
    assert_eq!(offset_of!(BbWorld, track), 16);
    assert_eq!(offset_of!(BbWorld, section), 24);
    assert_eq!(offset_of!(BbWorld, set_rain), 32);
    assert_eq!(offset_of!(BbWorld, set_traffic), 40);
}

#[test]
fn menus_table_and_row() {
    assert_eq!(size_of::<BbMenuRow>(), 72);
    assert_eq!(offset_of!(BbMenuRow, name), 8);
    assert_eq!(offset_of!(BbMenuRow, label), 24);
    assert_eq!(offset_of!(BbMenuRow, min), 40);
    assert_eq!(offset_of!(BbMenuRow, default), 52);
    assert_eq!(offset_of!(BbMenuRow, choices), 56);
    assert_eq!(offset_of!(BbMenuRow, choice_count), 64);
    assert_eq!(size_of::<BbMenus>(), 48);
    assert_eq!(offset_of!(BbMenus, ctx), 8);
    assert_eq!(offset_of!(BbMenus, register), 16);
    assert_eq!(offset_of!(BbMenus, unregister), 24);
    assert_eq!(offset_of!(BbMenus, get), 32);
    assert_eq!(offset_of!(BbMenus, set), 40);
    assert_eq!([BB_MENU_TOGGLE, BB_MENU_SLIDER, BB_MENU_LIST, BB_MENU_BUTTON], [0, 1, 2, 3]);
}

#[test]
fn commands_table() {
    assert_eq!(size_of::<BbCommands>(), 40);
    assert_eq!(align_of::<BbCommands>(), 8);
    assert_eq!(offset_of!(BbCommands, head), 0);
    assert_eq!(offset_of!(BbCommands, ctx), 8);
    assert_eq!(offset_of!(BbCommands, run), 16);
    assert_eq!(offset_of!(BbCommands, register), 24);
    assert_eq!(offset_of!(BbCommands, unregister), 32);
}

#[test]
fn input_table() {
    assert_eq!(size_of::<BbInput>(), 32);
    assert_eq!(offset_of!(BbInput, ctx), 8);
    assert_eq!(offset_of!(BbInput, register_hotkey), 16);
    assert_eq!(offset_of!(BbInput, unregister), 24);
}

#[test]
fn time_table_and_clock() {
    assert_eq!(size_of::<BbTime>(), 48);
    assert_eq!(offset_of!(BbTime, get), 16);
    assert_eq!(offset_of!(BbTime, set_hour), 24);
    assert_eq!(offset_of!(BbTime, set_day_length), 32);
    assert_eq!(offset_of!(BbTime, freeze), 40);
    assert_eq!(size_of::<BbTimeOfDay>(), 12);
    assert_eq!(offset_of!(BbTimeOfDay, frozen), 8);
    assert_eq!(offset_of!(BbTimeOfDay, lights_on), 9);
}

#[test]
fn map_table_and_blip() {
    assert_eq!(size_of::<BbMap>(), 40);
    assert_eq!(offset_of!(BbMap, add), 16);
    assert_eq!(offset_of!(BbMap, update), 24);
    assert_eq!(offset_of!(BbMap, remove), 32);
    assert_eq!(size_of::<BbRgba>(), 4);
    assert_eq!(size_of::<BbBlip>(), 40);
    assert_eq!(offset_of!(BbBlip, active), 4);
    assert_eq!(offset_of!(BbBlip, style), 8);
    assert_eq!(offset_of!(BbBlip, position), 24);
}

#[test]
fn settings_table_and_option() {
    assert_eq!(IFACE_SETTINGS, "bb.settings");
    assert_eq!(IFACE_SETTINGS_VERSION, (1, 0));
    assert_eq!(size_of::<BbSettingValue>(), 32);
    assert_eq!(offset_of!(BbSettingValue, label), 16);
    assert_eq!(size_of::<BbSettingOption>(), 72);
    assert_eq!(offset_of!(BbSettingOption, menu), 8);
    assert_eq!(offset_of!(BbSettingOption, id), 16);
    assert_eq!(offset_of!(BbSettingOption, label), 32);
    assert_eq!(offset_of!(BbSettingOption, values), 48);
    assert_eq!(offset_of!(BbSettingOption, value_count), 56);
    assert_eq!(offset_of!(BbSettingOption, default_on), 64);
    assert_eq!(size_of::<BbSettings>(), 40);
    assert_eq!(offset_of!(BbSettings, ctx), 8);
    assert_eq!(offset_of!(BbSettings, add_option), 16);
    assert_eq!(offset_of!(BbSettings, remove_option), 24);
    assert_eq!(offset_of!(BbSettings, get), 32);
    assert_eq!([BB_SETTING_TOGGLE, BB_SETTING_LIST, BB_SETTING_MENU_GAMEPLAY], [0, 1, 0]);
}

#[test]
fn config_table_and_sources() {
    assert_eq!(IFACE_CONFIG, "bb.config");
    assert_eq!(IFACE_CONFIG_VERSION, (1, 0));
    assert_eq!(size_of::<BbConfig>(), 32);
    assert_eq!(align_of::<BbConfig>(), 8);
    assert_eq!(offset_of!(BbConfig, head), 0);
    assert_eq!(offset_of!(BbConfig, ctx), 8);
    assert_eq!(offset_of!(BbConfig, read), 16);
    assert_eq!(offset_of!(BbConfig, sources), 24);
    assert_eq!([BB_CONFIG_FILE, BB_CONFIG_MAIN], [1, 2]);
}
