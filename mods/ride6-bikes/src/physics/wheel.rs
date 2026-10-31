//! `[bike.front]` and `[bike.rear]` with their `tyre` tables: the wheel, its radial stiffness and the Magic Formula 94
//! coefficients of the road row of its grip table (`docs/formats/ride6-physics-tyre.md`).

use anyhow::{Result, bail};

use super::toml_text::Toml;
use super::units::{metres, metres_per_second};
use super::vehicle::Vehicle;
use super::{Side, TyreGeometry};

/// The coefficient names in the order of `blackbox_modding`'s `Tyre`: lateral `a0..a10, a111, a112, a12, a13`.
fn lateral_names() -> Vec<String> {
    let mut names: Vec<String> = (0..=10).map(|i| format!("A{i}")).collect();
    names.extend(["A111", "A112", "A12", "A13"].map(str::to_owned));
    names
}

fn numbered(letter: char, count: usize) -> Vec<String> {
    (0..count).map(|i| format!("{letter}{i}")).collect()
}

pub fn write(t: &mut Toml, vehicle: &Vehicle, side: Side, geometry: &TyreGeometry) -> Result<()> {
    let wheel = vehicle.props().req(side.wheel_key())?;
    let row = vehicle.road_row(side.wheel_key())?;
    if geometry.width_m <= 0.0 {
        bail!("the {} tyre has no width (the part table has no size code for it)", side.name());
    }
    if !wheel.req("ShapeIsToroid")?.flag()? {
        bail!("{}: a wheel that is not a torus is in no RIDE 6 bike and has no key in the manifest", wheel.path());
    }
    let table = format!("bike.{}", side.name());
    t.table(&table);
    // Radius and width are not in the data: "set from the wheel mesh at spawn" (ride6-physics-vehicle.md).
    t.num("radius_m", geometry.radius_m);
    t.num("width_m", geometry.width_m);
    t.num("toroid_factor", wheel.f("ShapeToroidFactor")?);
    t.num("mass_kg", wheel.f("Mass")?);
    t.num("inertia_kgm2", wheel.f("Inertia")?);
    t.nums(
        "rolling",
        &[
            row.f("RollingResistanceConstant")?,
            row.f("RollingResistanceVelocity")?,
            row.f("RollingResistanceSqrVelocity")?,
        ],
    );
    t.num("rolling_slip", row.f("RollingResistanceSlip")?);
    t.num("rolling_slip_linearity", row.f("RollingResistanceSlipLinearity")?);
    t.num("lateral_grip_limit", wheel.f("LateralGripLimit")?);
    t.num("tire_stiffness_n_per_m", wheel.f("TireStiffness")?);
    t.num("max_tire_deformation_m", metres(wheel.f("MaxTireDeformation")?));
    t.num("compression_shape_factor", wheel.f("TireCompressionShapeFactor")?);
    t.num("slip_correction", wheel.f("SlipCorrection")?);
    t.num("rest_slip_correction", wheel.f("RestSlipCorrection")?);
    t.num("rest_speed_threshold_mps", metres_per_second(wheel.f("RestSpeedThreshold")?));

    t.table(&format!("{table}.tyre"));
    for (key, names) in [("a", lateral_names()), ("b", numbered('B', 11)), ("c", numbered('C', 18))] {
        let mut values = Vec::new();
        for name in &names {
            values.push(row.f(name)?);
        }
        t.nums(key, &values);
    }
    Ok(())
}
