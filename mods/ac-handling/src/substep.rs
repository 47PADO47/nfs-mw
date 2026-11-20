//! One substep of the wheel (docs/specs/ac-handling/wheel-dynamics.md sections 4.1 to 4.5): the slips without singularities,
//! the relaxation lag, the linearly implicit wheel spin step, the brake and rolling resistance clamp, and the force at the
//! end of the substep.

use crate::constants::{FZ0, RELAX_LENGTH_FLOOR, SLOPE_STEP};
use crate::params::TyreParams;
use crate::tyre_force::{ForceIn, ForceOut, tyre_force};
use crate::wheel::{WheelInput, WheelState};

/// What one tick holds constant while its substeps run (section 4.9).
pub struct Frame<'a> {
    /// The axle's tyre.
    pub p: &'a TyreParams,
    /// The host's query of this tick.
    pub i: &'a WheelInput,
    /// The wheel inertia after `inertia_scale`, kg m^2.
    pub inertia: f64,
    /// `hypot(Vx, Vy)`: the patch speed.
    pub speed: f64,
    /// The substep length, s.
    pub h: f64,
}

/// The instantaneous slips of a spin (section 4.1) and the floor the longitudinal one divides by.
#[derive(Debug, Clone, Copy)]
pub struct Slips {
    /// Longitudinal denominator `max(|R w|, v_low_x)`.
    pub wx: f64,
    /// Longitudinal theoretical slip.
    pub sx: f64,
    /// Lateral theoretical slip.
    pub sy: f64,
}

impl Frame<'_> {
    /// The slips at wheel spin `spin`.
    pub fn slips(&self, spin: f64) -> Slips {
        let rw = self.i.radius * spin;
        let wx = rw.abs().max(self.p.v_low_x);
        let wy = rw.abs().max(self.p.v_low_y);
        Slips { wx, sx: (rw - self.i.vx) / wx, sy: self.i.vy / wy }
    }

    /// The tyre force at the relaxed slips `(sx, sy)` with the wheel turning at `spin`.
    pub fn force_at(&self, sx: f64, sy: f64, spin: f64) -> ForceOut {
        let i = self.i;
        let slide_speed = (i.radius * spin - i.vx).hypot(i.vy);
        let input = ForceIn {
            radius: i.radius,
            fz: i.fz,
            sx,
            sy,
            gamma: 0.0,
            slide_speed,
            speed: self.speed,
            surf_lat: i.surf_lat,
            surf_drive: i.surf_drive,
            grip: i.grip,
        };
        tyre_force(self.p, &input)
    }

    /// The slope of `fx` in `sx` (central difference), never negative.
    pub fn slope(&self, sx: f64, sy: f64, spin: f64) -> f64 {
        let up = self.force_at(sx + SLOPE_STEP, sy, spin).fx;
        let down = self.force_at(sx - SLOPE_STEP, sy, spin).fx;
        ((up - down) / (2.0 * SLOPE_STEP)).max(0.0)
    }

    /// The backward Euler factor of the relaxation lag for a substep (section 4.2).
    pub fn relaxation(&self, n_prev: f64, rw: f64) -> f64 {
        let p = self.p;
        let fr = self.i.fz / FZ0;
        let grip_length = p.relax_len * (1.0 + p.relax_load_slope * (fr - 1.0)).max(RELAX_LENGTH_FLOOR);
        let length = grip_length * (1.0 + (p.relax_slide_ratio - 1.0) * n_prev.clamp(0.0, 1.0));
        let rate = (self.speed.max(rw.abs()) / length).max(1.0 / p.relax_tau_max);
        self.h * rate / (1.0 + self.h * rate)
    }

    /// Advance the wheel by one substep from spin `spin`: the new spin and the force at the end of the substep. The relaxed
    /// slips and `n_prev` of `state` are updated.
    pub fn step(&self, state: &mut WheelState, spin: f64) -> (f64, ForceOut) {
        let i = self.i;
        let r = i.radius;
        let rw = r * spin;
        let slips = self.slips(spin);
        let a = self.relaxation(state.n_prev, rw);
        // 4.3 linearly implicit wheel and slip
        let f0 = self.force_at(state.sx_r, state.sy_r, spin);
        let s = self.slope(state.sx_r, state.sy_r, spin);
        let numerator = self.h * (i.drive_torque - r * f0.fx - r * s * a * (slips.sx - state.sx_r));
        let denominator = self.inertia + self.h * r * r * s * a / slips.wx;
        let free = spin + numerator / denominator;
        // 4.4 brake and rolling resistance clamp
        let t_rr = r * i.fz * (self.p.rr_c0 + self.p.rr_c2 * rw * rw) * i.surf_rolling;
        let removable = self.h * (i.brake_torque + t_rr) / self.inertia;
        let new_spin = if free.abs() <= removable { 0.0 } else { free - free.signum() * removable };
        // 4.5 end of the substep
        let sx_end = (r * new_spin - i.vx) / slips.wx;
        state.sx_r += a * (sx_end - state.sx_r);
        state.sy_r += a * (slips.sy - state.sy_r);
        let end = self.force_at(state.sx_r, state.sy_r, new_spin);
        state.n_prev = end.n;
        (new_spin, end)
    }
}
