//! The `chassis` class: suspension, geometry and the aerodynamic coefficients
//! (`docs/specs/vehicle-suspension-tires.md` §1 and §2, `vehicle-steering-assists-aero.md` §5).

use blackbox_vehicle::aero::AeroSpec;
use blackbox_vehicle::suspension::ChassisSpec;

use super::fields::Fields;

pub fn chassis(c: Fields<'_>) -> ChassisSpec {
    ChassisSpec {
        spring_stiffness: c.axle_pair("SPRING_STIFFNESS"),
        spring_progression: c.axle_pair("SPRING_PROGRESSION"),
        shock_stiffness: c.axle_pair("SHOCK_STIFFNESS"),
        shock_ext_stiffness: c.axle_pair("SHOCK_EXT_STIFFNESS"),
        shock_valving: c.axle_pair("SHOCK_VALVING"),
        shock_digression: c.axle_pair("SHOCK_DIGRESSION"),
        shock_blowout: c.f32("SHOCK_BLOWOUT"),
        swaybar_stiffness: c.axle_pair("SWAYBAR_STIFFNESS"),
        travel: c.axle_pair("TRAVEL"),
        ride_height: c.axle_pair("RIDE_HEIGHT"),
        track_width: c.axle_pair("TRACK_WIDTH"),
        wheel_base: c.f32("WHEEL_BASE"),
        front_axle: c.f32("FRONT_AXLE"),
        front_weight_bias: c.f32("FRONT_WEIGHT_BIAS"),
        roll_center: c.f32("ROLL_CENTER"),
    }
}

pub fn aero(c: Fields<'_>) -> AeroSpec {
    AeroSpec {
        drag_coefficient: c.f32("DRAG_COEFFICIENT"),
        aero_coefficient: c.f32("AERO_COEFFICIENT"),
        aero_cg: c.f32("AERO_CG"),
        ..AeroSpec::default()
    }
}
