//! Reading the camera container of a car bundle. Layouts and offsets: `docs/formats/chameleon-camera.md`.
//!
//! The container is one Genesys object (`Genesys.Gen.VehicleCameraContainer`) that holds the bonnet, bumper and
//! external cameras inline, with the globals, the gradient adjustments, the rear view and the shake effect nested
//! in them. Each structure is found by the offset its type import gives; the field offsets are those of the
//! `GenesysType` resources (the same in all 93 cars **[verified]**).

use chameleon_bundle::Bundle;

use crate::error::{Error, Result};
use crate::genesys::{Object, TypeNames, find_objects};
use crate::params::{
    AccelMovement, CameraRoll, CarCameras, DecelPitch, ExternalCamera, ExternalGlobals, GradientAdjustments,
    HighSpeedShake, ImpactShake, Lens, MountedCamera, RearView, SpeedMovement, Vec3, YawSpring, from_data_axes,
};

const CONTAINER: &str = "Genesys.Gen.VehicleCameraContainer";
const EXTERNAL: &str = "Genesys.Gen.CameraGameplayExternal";
const BONNET: &str = "Genesys.Gen.CameraGameplayBonnet";
const BUMPER: &str = "Genesys.Gen.CameraGameplayBumper";
const GLOBALS: &str = "Genesys.Gen.CameraGameplayExternalGlobals";
const IMPACT: &str = "Genesys.Gen.CameraGameplayExternalGlobals.Impact_shake";
const LENS: &str = "Genesys.Gen.CameraGameplayExternalGlobals.LensProperties";
const GRADIENT: &str = "Genesys.Gen.CameraGameplayGradientAdjustments.Params";
const SPEED: &str = "Genesys.Gen.CameraGameplayExternal.Speed_based_movement";
const SHAKE: &str = "Genesys.Gen.CameraGameplayExternal.Speed_based_movement.High_Speed_Shake";
const ACCEL: &str = "Genesys.Gen.CameraGameplayExternal.Acceleration_movement";
const ROLL: &str = "Genesys.Gen.CameraGameplayExternal.Camera_Roll";
const YAW: &str = "Genesys.Gen.CameraGameplayExternal.Yaw_spring_modification";
const DECEL: &str = "Genesys.Gen.CameraGameplayExternal.Deceleration_Pitch_Change";
const HEIGHT: &str = "Genesys.Gen.CameraGameplayExternal.Speed_based_height_change";
const REAR: &str = "Genesys.Gen.CameraRearView";
const REAR_GLOBALS: &str = "Genesys.Gen.CameraRearViewGlobals";

/// The cameras of the car in `bundle` (a `VEH_<n>_HI.BNDL`): the first camera container it holds.
pub fn read_cameras(bundle: &Bundle) -> Result<CarCameras> {
    let mut all = read_all_cameras(bundle)?;
    if all.is_empty() {
        return Err(Error::Spec("the bundle has no camera container".into()));
    }
    Ok(all.swap_remove(0).1)
}

/// Every camera container of `bundle`, with the key of the object that holds it. A car bundle has one;
/// `VEHICLES/TRAFFICATTRIBS.BNDL` has eleven (one per traffic vehicle).
pub fn read_all_cameras(bundle: &Bundle) -> Result<Vec<(u64, CarCameras)>> {
    let types = TypeNames::read(bundle)?;
    let objects = find_objects(bundle, &types, CONTAINER)?;
    objects.into_iter().map(|(id, object)| Ok((id, parse_container(&object)?))).collect()
}

/// A host-space vector from three data-space floats at `at`.
fn vec3(object: &Object, at: usize) -> Result<Vec3> {
    Ok(from_data_axes(object.floats::<3>(at)?))
}

/// Decode the container in `object`.
pub(crate) fn parse_container(object: &Object) -> Result<CarCameras> {
    let container = object.at(CONTAINER)?;
    let cameras = CarCameras {
        external: external(object)?,
        globals: globals(object)?,
        gradient: gradient(object)?,
        bonnet: mounted(object, BONNET)?,
        bumper: mounted(object, BUMPER)?,
        rear_view: rear_view(object)?,
        object_key: object.u32(container + 0xC)?,
    };
    check(&cameras)?;
    Ok(cameras)
}

fn mounted(object: &Object, name: &str) -> Result<MountedCamera> {
    let at = object.at(name)?;
    let f = |offset: usize| object.f32(at + offset);
    Ok(MountedCamera {
        response: vec3(object, at + 0xC)?,
        eye: vec3(object, at + 0x18)?,
        target: vec3(object, at + 0x24)?,
        smoothing_s: f(0x34)?,
        sway_m: f(0x38)?,
        wind_shake: f(0x3C)?,
        fov_deg: [f(0x64)?, f(0x68)?],
        look_limit_deg: [f(0x58)?, f(0x5C)?],
        other: [f(0x40)?, f(0x44)?, f(0x48)?, f(0x4C)?, f(0x50)?, f(0x54)?, f(0x60)?, f(0x6C)?],
        object_key: object.u32(at + 0x30)?,
    })
}

fn external(object: &Object) -> Result<ExternalCamera> {
    let at = object.at(EXTERNAL)?;
    let f = |offset: usize| object.f32(at + offset);
    Ok(ExternalCamera {
        response: vec3(object, at + 0x124)?,
        focus: vec3(object, at + 0x130)?,
        ground_pivot: vec3(object, at + 0x13C)?,
        position: vec3(object, at + 0x148)?,
        fov_deg: [f(0x160)?, f(0x164)?],
        param_168: f(0x168)?,
        param_16c: f(0x16C)?,
        lag_s: f(0x170)?,
        speed_movement: speed_movement(object)?,
        accel_movement: accel_movement(object)?,
        roll: roll(object)?,
        yaw_spring: yaw_spring(object)?,
        decel_pitch: decel_pitch(object)?,
        height_at_speed: object.f32(object.at(HEIGHT)? + 0x10)?,
    })
}

fn speed_movement(object: &Object) -> Result<SpeedMovement> {
    let at = object.at(SPEED)?;
    let shake = object.at(SHAKE)?;
    Ok(SpeedMovement {
        back_at_speed: object.f32(at + 0x34)?,
        height_a: object.f32(at + 0x38)?,
        height_b: object.f32(at + 0x3C)?,
        param_40: object.f32(at + 0x40)?,
        shake: HighSpeedShake {
            amplitude: object.f32(shake + 0x10)?,
            rate: object.f32(shake + 0x14)?,
            full_speed_mph: object.f32(shake + 0x18)?,
            start_speed_mph: object.f32(shake + 0x1C)?,
        },
    })
}

fn accel_movement(object: &Object) -> Result<AccelMovement> {
    let at = object.at(ACCEL)?;
    let [a, b, c, d, e, f, g, h, i, j] = object.floats::<10>(at + 0x10)?;
    Ok(AccelMovement {
        pitch_deg: a,
        smoothing_s: b,
        param_18: c,
        param_1c: d,
        full_accel: e,
        back_m: f,
        forward_m: g,
        param_2c: h,
        param_30: i,
        param_34: j,
    })
}

fn roll(object: &Object) -> Result<CameraRoll> {
    let at = object.at(ROLL)?;
    let [a, b, c, d, e, f] = object.floats::<6>(at + 0x10)?;
    Ok(CameraRoll { param_10: a, param_14: b, full_speed_mph: c, max_deg: d, smoothing_s: e, param_24: f })
}

fn yaw_spring(object: &Object) -> Result<YawSpring> {
    let at = object.at(YAW)?;
    let [a, b, c, d, e, f] = object.floats::<6>(at + 0x10)?;
    Ok(YawSpring { speed_min: a, rate_low: b, speed_max: c, lead_s: d, rate_high: e, param_24: f })
}

fn decel_pitch(object: &Object) -> Result<DecelPitch> {
    let at = object.at(DECEL)?;
    let [a, b, c, d] = object.floats::<4>(at + 0x10)?;
    Ok(DecelPitch { rate_deg_s: a, max_deg: b, param_18: c, response: d })
}

fn globals(object: &Object) -> Result<ExternalGlobals> {
    object.at(GLOBALS)?;
    let impact = object.at(IMPACT)?;
    let lens = object.at(LENS)?;
    Ok(ExternalGlobals {
        impact_shake: ImpactShake {
            frequency: object.floats::<3>(impact + 0xC)?,
            amplitude_deg: object.floats::<3>(impact + 0x18)?,
            decay_s: object.floats::<3>(impact + 0x24)?,
            scalars: object.floats::<9>(impact + 0x34)?,
        },
        lens: Lens { fov_deg: object.f32(lens + 0x10)?, scale: object.f32(lens + 0x14)? },
    })
}

/// The two `Params` of the gradient adjustments, in memory order.
fn gradient(object: &Object) -> Result<GradientAdjustments> {
    let mut params = object.structs.iter().filter(|(name, _)| name == GRADIENT).map(|(_, offset)| *offset);
    let (Some(first), Some(second)) = (params.next(), params.next()) else {
        return Err(Error::Spec("the gradient adjustments need two params".into()));
    };
    Ok(GradientAdjustments { pitch_follow: [object.f32(first + 0x10)?, object.f32(second + 0x10)?] })
}

fn rear_view(object: &Object) -> Result<RearView> {
    let at = object.at(REAR)?;
    let globals = object.at(REAR_GLOBALS)?;
    Ok(RearView {
        offset: vec3(object, at + 0xC)?,
        param_1c: object.f32(at + 0x1C)?,
        aspect: object.f32(globals + 0x10)?,
        width_px: object.f32(globals + 0x14)?,
        fov_deg: object.f32(globals + 0x18)?,
    })
}

/// Refuse numbers that cannot be a camera, so a misread layout fails here and not on screen.
fn check(cameras: &CarCameras) -> Result<()> {
    let bad = |what: &str| Err(Error::Spec(format!("implausible camera data: {what}")));
    let external = &cameras.external;
    let all_finite = [external.position, external.focus, cameras.bonnet.eye, cameras.bumper.eye]
        .iter()
        .flatten()
        .all(|value| value.is_finite());
    if !all_finite {
        return bad("a non-finite vector");
    }
    let in_range = |value: f32, low: f32, high: f32| (low..=high).contains(&value);
    if !in_range(external.position[0], -30.0, -0.5) || !in_range(external.position[2], 0.2, 10.0) {
        return bad("chase camera position");
    }
    if !external.fov_deg.iter().all(|&fov| in_range(fov, 20.0, 170.0)) {
        return bad("chase camera field of view");
    }
    if !cameras.bonnet.fov_deg.iter().chain(&cameras.bumper.fov_deg).all(|&fov| in_range(fov, 20.0, 170.0)) {
        return bad("mounted camera field of view");
    }
    if !in_range(external.speed_movement.shake.full_speed_mph, 10.0, 600.0) {
        return bad("full speed");
    }
    if !in_range(cameras.rear_view.aspect, 0.5, 20.0) || !in_range(cameras.rear_view.fov_deg, 10.0, 170.0) {
        return bad("rear view");
    }
    Ok(())
}
