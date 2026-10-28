//! `[bike.front_suspension]` and `[bike.rear_suspension]` with their damper, progressive, hydraulic and coupling tables
//! (`docs/specs/ride6-suspension.md`; the units are those of the data: metres, N/m, N s/m, m/s).

use anyhow::Result;

use super::curve::{Translator, runtime_curve, simplify};
use super::node::Node;
use super::toml_text::Toml;
use super::vehicle::Vehicle;
use super::{Side, defaults};

/// The manifest's leverage curve holds up to 16 points.
const MAX_LEVERAGE_POINTS: usize = 16;

pub fn write(t: &mut Toml, vehicle: &Vehicle, side: Side) -> Result<()> {
    let props = vehicle.props();
    let s = props.req(side.suspension_key())?;
    let name = format!("bike.{}_suspension", side.name());
    t.table(&name);
    t.num("max_travel_m", s.f("MaxCompression")?);
    t.num("spring_n_per_m", s.f("SpringStrength")?);
    t.num("top_out_stiffness_n_per_m", s.f("TopOutSpringStiffness")?);
    t.num("top_out_length_m", s.f("TopOutSpringLength")?);
    t.num("rest_offset_m", s.f("RestCompressionOffset")?);
    t.num("max_speed_mps", s.f("SpringMaxSpeed")?);
    if let Some(points) = leverage(vehicle, &s)? {
        t.pairs("leverage", &points);
    }

    t.table(&format!("{name}.damper"));
    t.num("slow_bump", s.f("DamperSlowBump")?);
    t.num("fast_bump", s.f("DamperFastBump")?);
    t.num("bump_threshold", s.f("DamperFastBumpThreshold")?);
    t.num("slow_rebound", s.f("DamperSlowRebound")?);
    t.num("fast_rebound", s.f("DamperFastRebound")?);
    t.num("rebound_threshold", s.f("DamperFastReboundThreshold")?);

    progressive(t, &name, &s)?;
    hydraulic(t, &name, &s)?;
    coupling(t, vehicle, &name, &s)
}

/// The wheel travel per shock travel against the share of the travel, never below 1 (the binary clamps it,
/// `ride6-suspension.md`, "Leverage ratio"). `None` for a spring that acts on the wheel directly.
fn leverage(vehicle: &Vehicle, s: &Node<'_>) -> Result<Option<Vec<[f64; 2]>>> {
    let Some(node) = s.get("SuspensionLeverageRatio") else { return Ok(None) };
    let Some(curve) = runtime_curve(vehicle, &node)? else { return Ok(None) };
    let points: Vec<[f64; 2]> = curve.polyline().into_iter().map(|[share, ratio]| [share, ratio.max(1.0)]).collect();
    Ok(Some(simplify(points, MAX_LEVERAGE_POINTS)))
}

/// The progressive term of the main spring: written only when its factor is not 0 (0 is a linear spring).
fn progressive(t: &mut Toml, name: &str, s: &Node<'_>) -> Result<()> {
    let factor = s.f_or("ProgressiveSpringFactor", defaults::PROGRESSIVE_FACTOR)?;
    if factor == 0.0 {
        return Ok(());
    }
    t.table(&format!("{name}.progressive"));
    t.num("start", s.f_or("LinearSpringCompressionRatio", defaults::PROGRESSIVE_START)?);
    t.num("factor", factor);
    t.num("linearity", s.f_or("ProgressiveSpringLinearity", defaults::PROGRESSIVE_LINEARITY)?);
    Ok(())
}

/// The hydraulic term: a second, progressive spring-like term.
fn hydraulic(t: &mut Toml, name: &str, s: &Node<'_>) -> Result<()> {
    let stiffness = s.f_or("HydraulicComponentStiffness", 0.0)?;
    if stiffness == 0.0 {
        return Ok(());
    }
    t.table(&format!("{name}.hydraulic"));
    t.num("stiffness_n_per_m", stiffness);
    let start = s.f_or("LinearHydraulicComponentCompressionRatio", defaults::PROGRESSIVE_START)?;
    if start != 0.0 {
        t.num("start", start);
    }
    t.num("factor", s.f_or("ProgressiveHydraulicComponentFactor", defaults::PROGRESSIVE_FACTOR)?);
    t.num("linearity", s.f_or("ProgressiveHydraulicComponentLinearity", defaults::PROGRESSIVE_LINEARITY)?);
    Ok(())
}

/// The value of a swingarm force's remap of the travel share at no compression and at full travel.
fn remap(vehicle: &Vehicle, s: &Node<'_>, key: &str) -> Result<[f64; 2]> {
    let translator = match s.get(key) {
        Some(node) => Translator::read(vehicle, &node)?,
        None => Translator::identity(),
    };
    let ends = [translator.at(defaults::REMAP_IDENTITY[0]), translator.at(defaults::REMAP_IDENTITY[1])];
    Ok(ends)
}

/// The chain pull, traction and brake coupling into the suspension (`ride6-suspension.md`, "Chain pull, traction and brake
/// coupling"): a table when at least one factor is not 0.
fn coupling(t: &mut Toml, vehicle: &Vehicle, name: &str, s: &Node<'_>) -> Result<()> {
    let terms = [
        ("chain", "ChainPullFactor", "ChainPullFactorCompressionFactor"),
        ("traction", "TractionInfluenceOnLoad", "TractionInfluenceOnLoadCompressionFactor"),
        ("brake", "BrakeTorqueCouplingFactor", "BrakeTorqueCouplingCompressionFactor"),
    ];
    let mut rows = Vec::new();
    for (key, factor_key, remap_key) in terms {
        let factor = s.f_or(factor_key, defaults::FORK_SWINGARM_FACTOR)?;
        if factor != 0.0 {
            rows.push((key, factor, remap(vehicle, s, remap_key)?));
        }
    }
    if rows.is_empty() {
        return Ok(());
    }
    t.table(&format!("{name}.coupling"));
    for (key, factor, range) in rows {
        t.num(key, factor);
        t.nums(&format!("{key}_range"), &range);
    }
    Ok(())
}
