//! The thermal seam: the `grip_scale(T_eff, P, w)` factor of docs/specs/ac-handling/tyre-force.md section 2 that tyre
//! temperature, pressure and wear put on the grip of a wheel. It multiplies the `grip` input of the tyre force with the player's
//! grip gain and the tyre condition (wheel-dynamics.md section 4); it is held for the whole tick.
//!
//! [`NoThermal`] answers 1 (the thermal unit is off); [`ThermalGrip`] keeps the four tyres of [`crate::thermal`].

use crate::thermal::{HudValues, ThermalParams, ThermalState, ThermalTick};

/// What the tyre hook knows of a wheel at the start of a tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GripInputs {
    /// Camber this tick, outward positive, rad (0 without the suspension hook).
    pub camber_rad: f64,
}

/// The thermal grip factor of the wheels of one car.
pub trait GripScale {
    /// The multiplier of wheel `wheel` (0 to 3) for this tick, at its start: 1 is as tuned. Must be finite; the caller treats
    /// anything else as 1.
    fn grip_scale(&mut self, wheel: usize, inputs: &GripInputs) -> f64;

    /// The end of the tick of wheel `wheel`: heat, cooling and wear. The default does nothing.
    fn end_tick(&mut self, _wheel: usize, _tick: &ThermalTick) {}

    /// Start every wheel cold at the ambient temperature `ambient_c`: called when the mod is switched on, when the thermal unit is
    /// switched on and when the car changes. The default does nothing.
    fn reset(&mut self, _ambient_c: f64) {}

    /// The HUD values of a wheel at camber `camber_rad`, when there is a thermal model.
    fn hud(&self, _wheel: usize, _camber_rad: f64) -> Option<HudValues> {
        None
    }
}

/// The thermal unit off: the factor is always 1.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoThermal;

impl GripScale for NoThermal {
    fn grip_scale(&mut self, _wheel: usize, _inputs: &GripInputs) -> f64 {
        1.0
    }
}

/// The thermal model: a state per wheel.
#[derive(Debug, Clone)]
pub struct ThermalGrip {
    params: ThermalParams,
    wheels: [ThermalState; 4],
    warm_start: bool,
}

impl ThermalGrip {
    /// A cold set of tyres at 20 C; `warm_start` begins every reset at the warm temperature instead of the ambient one.
    pub fn new(params: ThermalParams, warm_start: bool) -> ThermalGrip {
        let state = ThermalState::seed(&params, 20.0, 20.0);
        ThermalGrip { params, wheels: [state.clone(), state.clone(), state.clone(), state], warm_start }
    }

    /// The constants in use.
    pub fn params(&self) -> &ThermalParams {
        &self.params
    }

    /// The state of wheel `wheel`.
    pub fn state(&self, wheel: usize) -> &ThermalState {
        &self.wheels[wheel.min(3)]
    }
}

impl GripScale for ThermalGrip {
    fn grip_scale(&mut self, wheel: usize, inputs: &GripInputs) -> f64 {
        self.wheels[wheel.min(3)].grip(&self.params, inputs.camber_rad)
    }

    fn end_tick(&mut self, wheel: usize, tick: &ThermalTick) {
        let params = &self.params;
        self.wheels[wheel.min(3)].step(params, tick);
    }

    fn reset(&mut self, ambient_c: f64) {
        let start = if self.warm_start { self.params.t_warm } else { ambient_c };
        let state = ThermalState::seed(&self.params, start, ambient_c);
        self.wheels = [state.clone(), state.clone(), state.clone(), state];
    }

    fn hud(&self, wheel: usize, camber_rad: f64) -> Option<HudValues> {
        Some(self.wheels[wheel.min(3)].hud(&self.params, camber_rad))
    }
}

/// A factor that is not a usable number counts as 1.
pub fn sanitize(factor: f64) -> f64 {
    if !factor.is_finite() || factor < 0.0 {
        return 1.0;
    }
    factor
}
