//! `[bike]` (drag, gyroscopic scales) and `[bike.chassis]`: mass, rider, inertia, geometry. Field mapping:
//! `docs/specs/vehicle-two-wheel-parameters.md`, section 1.

use anyhow::{Result, anyhow, bail};

use super::toml_text::Toml;
use super::units::{metres, radians, to_physics_space};
use super::vehicle::Vehicle;
use super::{Geometry, defaults};

/// The input handler of the vehicle: it holds the rider, the lean controller, the gearbox logic and the fall limits.
pub fn handler(vehicle: &Vehicle) -> Result<super::node::Node<'_>> {
    vehicle.element("InputHandler")?.ok_or_else(|| anyhow!("the vehicle runs no input handler element"))
}

/// A vector of RIDE 6 centimetres (x forward, y right, z up) as metres in the physics space of the manifest.
fn vector_m(ride6_cm: [f64; 3]) -> [f64; 3] {
    let v = to_physics_space(ride6_cm);
    [metres(v[0]), metres(v[1]), metres(v[2])]
}

pub fn write(t: &mut Toml, vehicle: &Vehicle, geometry: &Geometry) -> Result<()> {
    let props = vehicle.props();
    let handler = handler(vehicle)?;
    t.table("bike");
    t.num("drag", props.req("Aerodynamic")?.f("DragCoefficient")?);
    let (roll, yaw) = match handler.req("SimulateGyroscopicEffects")?.flag()? {
        true => (
            handler.f_or("GyroscopicRollMomentumScale", defaults::GYRO_SCALE)?,
            handler.f_or("GyroscopicYawMomentumScale", defaults::GYRO_SCALE)?,
        ),
        false => (0.0, 0.0),
    };
    t.num("gyro_roll_scale", roll);
    t.num("gyro_yaw_scale", yaw);

    let chassis = props.req("CHASSIS")?;
    t.table("bike.chassis");
    t.num("mass_kg", chassis.f("Mass")?);
    rider(t, vehicle)?;
    t.nums("half_dimensions_m", &geometry.half_m);
    // The data's scales are (x roll, y pitch, z yaw); the manifest's are (pitch, yaw, roll).
    let tensor = chassis.req("InertiaTensorScale")?.xyz()?;
    t.nums("tensor_scale", &[tensor[1], tensor[2], tensor[0]]);
    t.num("cg_height_m", metres(chassis.req("CGPosition")?.f("Z")?));
    t.num("front_weight", chassis.f("WeightBalance")?);

    let skeleton = props.req("Skeleton")?;
    if !skeleton.flag_or("bOverrideSkeletonData", false)? {
        bail!("{}: the geometry comes from the skeleton, which the importer does not read", skeleton.path());
    }
    let wheelbase = metres(skeleton.f("OverrideWheelBase")?);
    t.num("wheelbase_m", wheelbase);
    let first = skeleton
        .req("RakeTrailOverride")?
        .items()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("{}: no rake and trail", skeleton.path()))?;
    t.num("rake_rad", radians(first.f("Rake")?));
    t.num("trail_m", metres(first.f("Trail")?));
    if (wheelbase - geometry.wheelbase()).abs() > 0.01 {
        t.note(format!(
            "the data's wheelbase ({wheelbase:.4} m) and the distance between the hubs of the meshes ({:.4} m) differ by \
             more than 1 cm; the dynamics use the data's, the model is drawn on the hubs",
            geometry.wheelbase()
        ));
    }
    Ok(())
}

/// The rider's mass and where it sits (`docs/specs/ride6-rider-body-aero.md`). `COMOffset` is from the bike's centre of
/// gravity (the rider's own height above the ground is `CGPosition.z + COMOffset.z`).
fn rider(t: &mut Toml, vehicle: &Vehicle) -> Result<()> {
    let handler = handler(vehicle)?;
    let rider = handler.req("Rider")?;
    if !rider.flag_or("SimulateRider", true)? {
        return Ok(());
    }
    t.num("rider_mass_kg", rider.f("Mass")?);
    t.nums("rider_offset_m", &vector_m(rider.req("COMOffset")?.xyz()?));
    t.nums("rider_forward_shift_m", &vector_m(rider.xyz_or("ForwardOffset", defaults::RIDER_FORWARD_OFFSET)?));
    t.nums("rider_backward_shift_m", &vector_m(rider.xyz_or("BackwardOffset", defaults::RIDER_BACKWARD_OFFSET)?));
    let weight = vehicle.props().req("InputData")?.req("WeightLongitudinal")?;
    if weight.flag_or("bUseDeprecatedRate", false)? {
        bail!("{}: the deprecated input rates are not supported", weight.path());
    }
    t.num("rider_weight_rate", weight.f("InputRate")?);
    t.num("rider_k_n_per_m", rider.f_or("RiderK", defaults::RIDER_K)?);
    t.num("rider_damp", rider.f_or("RiderDamp", defaults::RIDER_DAMP)?);
    Ok(())
}
