//! `[bike.engine]`: peak torque and its curve, the closed-throttle drag, idle and rev limit
//! (`docs/specs/ride6-engine-drivetrain.md`, "Engine torque").

use anyhow::{Result, anyhow};

use super::curve::{runtime_curve, simplify};
use super::engine_brake::BrakeReduction;
use super::node::Node;
use super::toml_text::Toml;
use super::vehicle::Vehicle;

/// The manifest's engine curves hold up to 32 points.
pub const MAX_CURVE_POINTS: usize = 32;

/// The engine as the gearbox generator needs it too.
pub struct EngineData {
    pub peak_torque: f64,
    /// `(rpm, share of the peak)` as the data has it, before the manifest's limit of points.
    pub full_curve: Vec<[f64; 2]>,
    pub friction_torque: f64,
    pub friction_curve: Vec<[f64; 2]>,
    pub idle_rpm: f64,
    pub max_rpm: f64,
    pub moi: f64,
    pub throttle_at_idle: f64,
    pub limiter_cut_time: f64,
    pub brake_reduction: BrakeReduction,
}

impl EngineData {
    pub fn read(vehicle: &Vehicle) -> Result<Self> {
        let props = vehicle.props();
        let engine = props.req("Engine")?;
        let max_rpm = engine.f("MaxRPM")?;
        // `MinRPM` is the idle speed; a few assets leave it to the base class and set only `MinRPMLimit`.
        let idle_rpm = match engine.get("MinRPM") {
            Some(node) => node.num()?,
            None => engine.f("MinRPMLimit")?,
        };
        let torque = runtime_curve(vehicle, &engine.req("TorqueCurve")?)?
            .ok_or_else(|| anyhow!("{}: the torque curve has no keys", engine.path()))?;

        let friction_curve = friction_points(vehicle, &engine, max_rpm)?;
        // The limiter that runs at the rev limit is entry 0 (same spec); its modulator cuts and restores the fuel for a
        // duration drawn from a range (a fixed range for every bike).
        let limiter = engine
            .req("EngineLimiter")?
            .items()?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("{}: no limiter", engine.path()))?;
        let [low, high] = limiter.req("LimiterModulatorTuning")?.xy_of("DurationRange")?;
        Ok(Self {
            peak_torque: engine.f("PeakTorque")?,
            full_curve: torque.polyline(),
            friction_torque: engine.f("EngineFrictionPeakTorque")?,
            friction_curve,
            idle_rpm,
            max_rpm,
            moi: engine.f("MOI")?,
            throttle_at_idle: engine.f("ThrottleAtMinimum")?,
            limiter_cut_time: (low + high) / 2.0,
            brake_reduction: BrakeReduction::read(vehicle, &engine)?,
        })
    }

    /// The torque curve within the manifest's limit of points: when the data has more, the keys beyond the rev limit go
    /// first (the limiter holds the engine under it), then the least significant ones.
    pub fn manifest_curve(&self) -> Vec<[f64; 2]> {
        let mut points = self.full_curve.clone();
        let cut = points.iter().position(|p| p[0] >= self.max_rpm).filter(|_| points.len() > MAX_CURVE_POINTS);
        if let Some(cut) = cut {
            points.truncate(cut + 1);
        }
        simplify(points, MAX_CURVE_POINTS)
    }
}

/// The closed-throttle drag against `rpm / max_rpm`; empty when the engine has no curve (the model then scales the drag
/// linearly with the rev limit).
fn friction_points(vehicle: &Vehicle, engine: &Node<'_>, max_rpm: f64) -> Result<Vec<[f64; 2]>> {
    let Some(node) = engine.get("EngineFrictionCurve") else { return Ok(Vec::new()) };
    let Some(curve) = runtime_curve(vehicle, &node)? else { return Ok(Vec::new()) };
    // The curve's domain is rpm / MaxRPM when normalised, else rpm (same spec).
    let scale = match engine.req("NormalizeEngineFrictionCurveDomain")?.flag()? {
        true => 1.0,
        false => 1.0 / max_rpm,
    };
    let points = curve.polyline().into_iter().map(|[x, y]| [x * scale, y]).collect();
    Ok(simplify(points, MAX_CURVE_POINTS))
}

pub fn write(t: &mut Toml, engine: &EngineData) {
    t.table("bike.engine");
    t.num("peak_torque_nm", engine.peak_torque);
    t.pairs("torque_curve", &engine.manifest_curve());
    if engine.full_curve.len() > MAX_CURVE_POINTS {
        t.note(format!(
            "the torque curve has {} keys; the manifest takes {MAX_CURVE_POINTS}, so the least significant were dropped",
            engine.full_curve.len()
        ));
    }
    t.num("friction_torque_nm", engine.friction_torque);
    if !engine.friction_curve.is_empty() {
        t.pairs("friction_curve", &engine.friction_curve);
    }
    t.num("idle_rpm", engine.idle_rpm);
    t.num("max_rpm", engine.max_rpm);
    t.num("moi_kgm2", engine.moi);
    t.num("throttle_at_idle", engine.throttle_at_idle);
    t.num("limiter_cut_time_s", engine.limiter_cut_time);
    engine.brake_reduction.write(t);
}
