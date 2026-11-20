//! Layout snapshot of `bb.handling` 1.0 and of the `bb.vehicles` 1.3 spec (ADR 0005 section 9). Changing a number here is an
//! ABI change.

#![cfg(target_pointer_width = "64")]

use core::mem::{align_of, offset_of, size_of};

use crate::abi::*;

#[test]
fn id_and_version_are_locked() {
    assert_eq!(IFACE_HANDLING, "bb.handling");
    assert_eq!(IFACE_HANDLING_VERSION, (1, 0));
    assert_eq!([BB_TYRE_HANDLED, BB_TYRE_NOT_HANDLED, BB_TYRE_FLAG_ALL_ROLES as i32], [1, 0, 1]);
}

#[test]
fn handling_table() {
    assert_eq!(size_of::<BbHandling>(), 32);
    assert_eq!(align_of::<BbHandling>(), 8);
    assert_eq!(offset_of!(BbHandling, head), 0);
    assert_eq!(offset_of!(BbHandling, ctx), 8);
    assert_eq!(offset_of!(BbHandling, register_tyre), 16);
    assert_eq!(offset_of!(BbHandling, unregister_tyre), 24);
}

#[test]
fn tyre_hook_registration() {
    assert_eq!(size_of::<BbTyreHook>(), 56);
    assert_eq!(align_of::<BbTyreHook>(), 8);
    assert_eq!(offset_of!(BbTyreHook, size), 0);
    assert_eq!(offset_of!(BbTyreHook, flags), 4);
    assert_eq!(offset_of!(BbTyreHook, user), 8);
    assert_eq!(offset_of!(BbTyreHook, callback), 16);
    assert_eq!(offset_of!(BbTyreHook, reserved), 24);
}

#[test]
fn tyre_query() {
    assert_eq!(size_of::<BbTyreQuery>(), 120);
    assert_eq!(align_of::<BbTyreQuery>(), 4);
    assert_eq!(offset_of!(BbTyreQuery, size), 0);
    assert_eq!(offset_of!(BbTyreQuery, wheel_index), 4);
    assert_eq!(offset_of!(BbTyreQuery, role), 8);
    assert_eq!(offset_of!(BbTyreQuery, loaded), 12);
    assert_eq!(offset_of!(BbTyreQuery, vehicle_revision), 16);
    assert_eq!(offset_of!(BbTyreQuery, surface_tag), 20);
    assert_eq!(offset_of!(BbTyreQuery, dt), 24);
    assert_eq!(offset_of!(BbTyreQuery, vertical_load_n), 28);
    assert_eq!(offset_of!(BbTyreQuery, patch_lateral_velocity), 32);
    assert_eq!(offset_of!(BbTyreQuery, patch_forward_velocity), 36);
    assert_eq!(offset_of!(BbTyreQuery, body_speed), 40);
    assert_eq!(offset_of!(BbTyreQuery, wheel_spin), 44);
    assert_eq!(offset_of!(BbTyreQuery, drive_torque_nm), 48);
    assert_eq!(offset_of!(BbTyreQuery, brake_torque_nm), 52);
    assert_eq!(offset_of!(BbTyreQuery, wheel_radius_m), 56);
    assert_eq!(offset_of!(BbTyreQuery, wheel_inertia_kgm2), 60);
    assert_eq!(offset_of!(BbTyreQuery, surface_lateral), 64);
    assert_eq!(offset_of!(BbTyreQuery, surface_drive), 68);
    assert_eq!(offset_of!(BbTyreQuery, surface_rolling), 72);
    assert_eq!(offset_of!(BbTyreQuery, tyre_condition), 76);
    assert_eq!(offset_of!(BbTyreQuery, ambient_temperature_c), 80);
    assert_eq!(offset_of!(BbTyreQuery, road_temperature_c), 84);
    assert_eq!(offset_of!(BbTyreQuery, corner_mass_kg), 88);
    assert_eq!(offset_of!(BbTyreQuery, reserved), 92);
}

#[test]
fn tyre_result() {
    assert_eq!(size_of::<BbTyreResult>(), 64);
    assert_eq!(align_of::<BbTyreResult>(), 4);
    assert_eq!(offset_of!(BbTyreResult, size), 0);
    assert_eq!(offset_of!(BbTyreResult, longitudinal_force_n), 4);
    assert_eq!(offset_of!(BbTyreResult, lateral_force_n), 8);
    assert_eq!(offset_of!(BbTyreResult, aligning_torque_nm), 12);
    assert_eq!(offset_of!(BbTyreResult, wheel_spin), 16);
    assert_eq!(offset_of!(BbTyreResult, traction), 20);
    assert_eq!(offset_of!(BbTyreResult, slip), 24);
    assert_eq!(offset_of!(BbTyreResult, slip_angle), 28);
    assert_eq!(offset_of!(BbTyreResult, reserved), 32);
}

#[test]
fn vehicle_spec() {
    assert_eq!(size_of::<BbVehicleSpec>(), 168);
    assert_eq!(align_of::<BbVehicleSpec>(), 4);
    assert_eq!(offset_of!(BbVehicleSpec, size), 0);
    assert_eq!(offset_of!(BbVehicleSpec, revision), 4);
    assert_eq!(offset_of!(BbVehicleSpec, mass_kg), 8);
    assert_eq!(offset_of!(BbVehicleSpec, wheelbase_m), 12);
    assert_eq!(offset_of!(BbVehicleSpec, track_front_m), 16);
    assert_eq!(offset_of!(BbVehicleSpec, track_rear_m), 20);
    assert_eq!(offset_of!(BbVehicleSpec, cg_height_m), 24);
    assert_eq!(offset_of!(BbVehicleSpec, front_weight_fraction), 28);
    assert_eq!(offset_of!(BbVehicleSpec, tyre_radius_front_m), 32);
    assert_eq!(offset_of!(BbVehicleSpec, tyre_radius_rear_m), 36);
    assert_eq!(offset_of!(BbVehicleSpec, tyre_width_front_m), 40);
    assert_eq!(offset_of!(BbVehicleSpec, tyre_width_rear_m), 44);
    assert_eq!(offset_of!(BbVehicleSpec, peak_friction_front), 48);
    assert_eq!(offset_of!(BbVehicleSpec, peak_friction_rear), 52);
    assert_eq!(offset_of!(BbVehicleSpec, sliding_friction_front), 56);
    assert_eq!(offset_of!(BbVehicleSpec, sliding_friction_rear), 60);
    assert_eq!(offset_of!(BbVehicleSpec, drive_layout), 64);
    assert_eq!(offset_of!(BbVehicleSpec, gear_count), 68);
    assert_eq!(offset_of!(BbVehicleSpec, max_power_w), 72);
    assert_eq!(offset_of!(BbVehicleSpec, max_power_rpm), 76);
    assert_eq!(offset_of!(BbVehicleSpec, redline_rpm), 80);
    assert_eq!(offset_of!(BbVehicleSpec, final_drive), 84);
    assert_eq!(offset_of!(BbVehicleSpec, gear_ratios), 88);
    assert_eq!(offset_of!(BbVehicleSpec, brake_torque_front_nm), 120);
    assert_eq!(offset_of!(BbVehicleSpec, brake_torque_rear_nm), 124);
    assert_eq!(offset_of!(BbVehicleSpec, drag_area_m2), 128);
    assert_eq!(offset_of!(BbVehicleSpec, downforce_area_m2), 132);
    assert_eq!(offset_of!(BbVehicleSpec, reserved), 136);
    assert_eq!([BB_DRIVE_FRONT, BB_DRIVE_REAR, BB_DRIVE_ALL], [0, 1, 2]);
    assert_eq!(BB_SPEC_GEARS, 8);
}
