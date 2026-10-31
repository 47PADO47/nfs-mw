//! `[bike.engine]` keys of the engine-brake reduction (`docs/specs/ride6-powertrain-round6.md`, section 11): the factor and the
//! load and slip translators. What the library does not model is refused.

use anyhow::{Result, bail};

use super::curve::Translator;
use super::node::Node;
use super::toml_text::Toml;
use super::vehicle::Vehicle;

/// The tunings as `[input low, input high, output low, output high]`.
pub struct BrakeReduction {
    pub factor: f64,
    /// The load and slip translators; `None` where the asset leaves one unset (the constructor default outputs 1).
    pub load: Option<[f64; 4]>,
    pub slip: Option<[f64; 4]>,
}

fn plain(vehicle: &Vehicle, engine: &Node<'_>, key: &str, absolute: bool) -> Result<Option<[f64; 4]>> {
    let Some(node) = engine.get(key) else { return Ok(None) };
    let Some(translator) = Translator::read_opt(vehicle, &node)? else { return Ok(None) };
    if !translator.is_plain() || node.flag_or("UseAbsoluteValue", false)? != absolute {
        bail!("{}: a curve, a linearity or an absolute value in an engine-brake tuning is not supported", node.path());
    }
    let [a, b] = translator.input_range();
    let [c, d] = translator.output_range();
    Ok(Some([a, b, c, d]))
}

impl BrakeReduction {
    pub fn read(vehicle: &Vehicle, engine: &Node<'_>) -> Result<Self> {
        let factor = engine.f_or("EngineBrakeReductionFactor", 0.0)?;
        // The constructor defaults the library takes: the older reduction, the friction limit 1, no lerp with the clutch, no lean
        // tuning (it outputs 1).
        let unsupported = !engine.flag_or("bUseDeprecatedEngineBrakeReduction", true)?
            || engine.f_or("FrictionPeakTorqueLimit", 1.0)? != 1.0
            || engine.flag_or("bLerpEngineFrictionSettings", false)?
            || engine.get("EngineBrakeReductionLeanTuning").is_some();
        if unsupported && factor != 0.0 {
            bail!(
                "{}: this engine-brake reduction (newer logic, limit, clutch lerp or lean tuning) is not supported",
                engine.path()
            );
        }
        Ok(Self {
            factor,
            load: plain(vehicle, engine, "EngineBrakeReductionLoadTuning", false)?,
            slip: plain(vehicle, engine, "EngineBrakeReductionSlipTuning", false)?,
        })
    }

    pub fn write(&self, t: &mut Toml) {
        if self.factor == 0.0 {
            return;
        }
        t.num("brake_reduction_factor", self.factor);
        if let Some(load) = &self.load {
            t.nums("brake_reduction_load", load);
        }
        if let Some(slip) = &self.slip {
            t.nums("brake_reduction_slip", slip);
        }
    }
}
