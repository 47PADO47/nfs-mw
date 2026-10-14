//! Live powertrain hardware replacement, retaining moving-car state.

use super::Powertrain;
use crate::{drivetrain::TransmissionSpec, engine::EngineSpec, induction::InductionSpec, nos::NosSpec};

impl Powertrain {
    /// Installs new hardware without respawning the vehicle. Rebuilds shift and peak maps, retains
    /// engine speed and unchanged induction/nitrous runtime state. Changed boost hardware clears its
    /// old boost; an existing tank retains its fraction. Pending drive torque is rebuilt next tick.
    pub fn reconfigure(
        &mut self,
        engine: EngineSpec,
        trans: TransmissionSpec,
        induction: InductionSpec,
        nos: NosSpec,
        radius: f32,
    ) {
        let old_ratio = self.trans.ratio(self.gear) * self.trans.final_gear;
        let old_max = self.engine.rev_limit_rad();
        let old_range = old_max - self.engine.idle_rad();
        let axle_speed = match old_ratio > 0.0 && old_range > 0.0 {
            true => (self.omega_trans - self.engine.idle_rad()) * old_max / (old_ratio * old_range),
            false => 0.0,
        };
        let mut next = Self::new(engine, trans, induction, nos, radius);
        next.gear = self.gear.min(next.top_gear());
        next.omega = self.omega.clamp(next.engine.idle_rad(), next.engine.rev_limit_rad());
        next.rpm_display = self.rpm_display.clamp(next.engine.idle, next.engine.rev_limit());
        let max = next.engine.rev_limit_rad();
        let ratio = next.trans.ratio(next.gear) * next.trans.final_gear;
        next.omega_trans = next.engine.idle_rad() + axle_speed * ratio * (max - next.engine.idle_rad()) / max;
        if self.gear == next.gear {
            next.clutch = self.clutch;
            next.shift_timer = self.shift_timer;
            next.sport_timer = self.sport_timer;
        }
        if self.induction_spec == induction {
            next.induction = self.induction;
        }
        match (self.nos_spec == nos, self.has_nos() && next.has_nos()) {
            (true, _) => next.nos = self.nos,
            (_, true) => next.nos.capacity = self.nos.capacity.clamp(0.0, 1.0),
            _ => {}
        }
        *self = next;
    }
}
