//! The brakes and tires classes of a car (`docs/specs/vehicle-input-induction-brakes.md` §9 and
//! `vehicle-suspension-tires.md` §3, "fields read").

use blackbox_vehicle::brakes::BrakeSpec;
use blackbox_vehicle::tires::TireSpec;

use super::fields::Fields;

pub fn brakes(c: Fields<'_>) -> BrakeSpec {
    BrakeSpec { brakes: c.axle_pair("BRAKES"), brake_lock: c.axle_pair("BRAKE_LOCK"), ebrake: c.f32("EBRAKE") }
}

pub fn tires(c: Fields<'_>) -> TireSpec {
    TireSpec {
        rim_size: c.axle_pair("RIM_SIZE"),
        section_width: c.axle_pair("SECTION_WIDTH"),
        aspect_ratio: c.axle_pair("ASPECT_RATIO"),
        grip_scale: c.axle_pair("GRIP_SCALE"),
        static_grip: c.axle_pair("STATIC_GRIP"),
        dynamic_grip: c.axle_pair("DYNAMIC_GRIP"),
        steering: c.f32("STEERING"),
        yaw_control: c.floats("YAW_CONTROL"),
        yaw_speed: c.f32("YAW_SPEED"),
    }
}
