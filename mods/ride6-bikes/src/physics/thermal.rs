//! `[bike.front.thermal]` and `[bike.rear.thermal]`: the temperature and wear of a tyre (`docs/specs/ride6-tyre-thermal.md`).
//! The decay data come from the tyre's `TireData`, the strips from its `TireSections`, the time scales and the uniform mode from
//! the vehicle's tyre-wear element. A tyre whose data lists no strips has no table.

use anyhow::Result;

use super::Side;
use super::curve::{runtime_curve, simplify};
use super::defaults::thermal as default;
use super::engine::MAX_CURVE_POINTS;
use super::node::Node;
use super::toml_text::Toml;
use super::units::metres;
use super::vehicle::Vehicle;

/// The points of a section's curve, simplified; empty when the section has none (the library counts a missing curve as 1).
fn curve(vehicle: &Vehicle, section: &Node<'_>, key: &str) -> Result<Vec<[f64; 2]>> {
    let Some(node) = section.get(key) else { return Ok(Vec::new()) };
    Ok(match runtime_curve(vehicle, &node)? {
        Some(curve) => simplify(curve.polyline(), MAX_CURVE_POINTS),
        None => Vec::new(),
    })
}

/// A coefficient that the data gives per dry and wet surface, taken at the dry end (a dry road) when the section interpolates by
/// wetness, else the plain coefficient.
fn dry_or_plain(section: &Node<'_>, plain: &str, dry: &str, interpolated: &str) -> Result<f64> {
    match section.flag_or(interpolated, false)? {
        true => section.f(dry),
        false => section.f(plain),
    }
}

pub fn write(t: &mut Toml, vehicle: &Vehicle, side: Side) -> Result<()> {
    let wheel = vehicle.props().req(side.wheel_key())?;
    let tyre = vehicle.target(&wheel.req("TireData")?)?;
    let Some(list) = tyre.get("TireSections") else { return Ok(()) };
    let references = list.items()?;
    if references.is_empty() {
        return Ok(());
    }
    let decay = tyre.get("Decay");
    let read = |key: &str, fallback: f64| match &decay {
        Some(node) => node.f_or(key, fallback),
        None => Ok(fallback),
    };
    let element = vehicle.element("TireWear")?;
    let setting = |key: &str, fallback: f64| match &element {
        Some(node) => node.f_or(key, fallback),
        None => Ok(fallback),
    };
    let uniform = match &element {
        Some(node) => node.flag_or("UniformTiresComputation", default::UNIFORM)?,
        None => default::UNIFORM,
    };

    t.table(&format!("bike.{}.thermal", side.name()));
    t.num("sections_dependency", read("TireSectionsDependency", default::SECTIONS_DEPENDENCY)?);
    t.num("sections_angle_rad", read("TireSectionsAngleRad", default::SECTIONS_ANGLE_RAD)?);
    t.num("heat_transfer", read("TireSectionsHeatTransfer", default::HEAT_TRANSFER)?);
    t.num("initial_thickness", read("Spess", default::SPESS)?);
    t.num("min_temperature_c", read("Temperature", default::TEMPERATURE)?);
    t.num("max_temperature_c", read("MaxTemperature", default::MAX_TEMPERATURE)?);
    t.num("asphalt_importance", read("AsphaltTemperatureImportance", default::ASPHALT_IMPORTANCE)?);
    t.num("wear_length_m", metres(read("NominalWearDecayLength", default::WEAR_LENGTH_CM)?));
    t.flag("uniform", uniform);
    t.num("static_time_scale", setting("StaticTemperatureTimeScale", default::STATIC_TIME_SCALE)?);
    t.num("dynamic_time_scale", setting("DynamicTemperatureTimeScale", default::DYNAMIC_TIME_SCALE)?);
    t.num("decay_distance_factor", setting("TireDecayDistanceFactor", default::DECAY_DISTANCE_FACTOR)?);
    t.num("wear_intensity", setting("WearDecayIntensity", default::WEAR_INTENSITY)?);

    for reference in references {
        let section = vehicle.target(&reference)?;
        t.table_array(&format!("bike.{}.thermal.sections", side.name()));
        t.num("efficiency_limit", section.f("EfficiencyLimit")?);
        t.num(
            "static_coef",
            dry_or_plain(
                &section,
                "StaticTemperatureCoef",
                "StaticTemperatureCoefDry",
                "bStaticTemperatureWetnessInterpolationEnabled",
            )?,
        );
        t.num(
            "dynamic_coef",
            dry_or_plain(
                &section,
                "DynamicTemperatureCoef",
                "DynamicTemperatureCoefDry",
                "bDynamicTemperatureWetnessInterpolationEnabled",
            )?,
        );
        t.num("lat_slip_coef", section.f("LatSlipToTemperatureCoef")?);
        t.num("long_slip_coef", section.f("LongSlipToTemperatureCoef")?);
        t.num("rolling_coef", section.f("RollingResistanceToTemperatureCoef")?);
        for (key, name) in [
            ("temperature_to_wear", "TemperatureToWearCoefficientCurve"),
            ("wear_to_efficiency", "WearToEfficiencyCurve"),
            ("temperature_to_efficiency", "TemperatureToEfficiencyCurve"),
        ] {
            let points = curve(vehicle, &section, name)?;
            if !points.is_empty() {
                t.pairs(key, &points);
            }
        }
    }
    Ok(())
}
