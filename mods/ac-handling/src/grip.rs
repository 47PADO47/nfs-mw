//! The thermal seam: `grip_scale(T_eff, P, w)` of docs/specs/ac-handling/tyre-force.md section 2, the factor that tyre
//! temperature, pressure and wear (stage 5, `tyre-thermal.md`) put on the grip of a wheel. Stage 2 has none: [`NoThermal`]
//! answers 1 for every wheel. The factor multiplies the `grip` input of the tyre force with the player's grip gain and the
//! tyre condition (section 4 of wheel-dynamics.md); it is held for the whole tick, like the other inputs.

/// What the thermal model needs to say how grippy a wheel is. The units are those of `tyre-thermal.md`; the stage-2 seam
/// fills the ambient temperature in `t_eff` and leaves the rest at 0.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GripInputs {
    /// The effective tyre temperature `T_eff`.
    pub t_eff: f64,
    /// The tyre pressure `P`.
    pub pressure: f64,
    /// The wear `w`.
    pub wear: f64,
}

/// The thermal grip factor of the wheels of one car. A stage-5 implementation keeps the per-wheel thermal state here and
/// advances it in `grip_scale`.
pub trait GripScale {
    /// The multiplier of wheel `wheel` (0 to 3) for this tick: 1 is as tuned. Must be finite; the caller treats anything
    /// else as 1.
    fn grip_scale(&mut self, wheel: usize, inputs: &GripInputs) -> f64;

    /// Start every wheel cold at the ambient temperature `ambient_c`: called when the mod is switched on and when the car
    /// changes. The default does nothing.
    fn reset(&mut self, _ambient_c: f64) {}
}

/// Stage 2: no thermal model, the factor is always 1.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoThermal;

impl GripScale for NoThermal {
    fn grip_scale(&mut self, _wheel: usize, _inputs: &GripInputs) -> f64 {
        1.0
    }
}

/// A factor that is not a usable number counts as 1.
pub fn sanitize(factor: f64) -> f64 {
    if !factor.is_finite() || factor < 0.0 {
        return 1.0;
    }
    factor
}
