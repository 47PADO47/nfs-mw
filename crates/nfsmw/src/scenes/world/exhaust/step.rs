//! One physics step of a loaded car's flames: the triggers, then every pipe's emitters.

use blackbox_particles::Frame;

use super::trigger::{self, blowoff_allowed, flame_intensity_with};
use super::{Active, CarState};

/// Whether the game's orange blow-off and backfire is not drawn: the nitrous burns, or a variant's pop effect stands in
/// for it. That effect plays only with the pedal up (`variant::PEDAL_DOWN`), so a flame under power (the shift blow-off, a
/// mod car's pop) stays the game's own: it would otherwise show nothing at all.
fn hides_orange(nitrous: bool, pop_replaced: bool, throttle: f32) -> bool {
    nitrous || (pop_replaced && throttle <= super::variant::PEDAL_DOWN)
}

impl Active {
    /// One physics step: the shift event and the backfire, then every pipe's emitters (spawning while the pipes
    /// flame). A step with nothing flaming and nothing alive does no more than that.
    pub(super) fn step(
        &mut self,
        dt: f32,
        car: &CarState,
        pops: u32,
        forced: bool,
        engine_level: i32,
        replaced: (bool, bool),
    ) {
        // A mod car shifts with a flame only up and under power (a lift-off has its pops), and flames for every pop its
        // sound makes, whatever the pedal does by the time the bang is due. A bike also flicks at a down-shift off the pedal.
        let bike = self.bike;
        let flames_at = |up: bool| match bike {
            true => trigger::bike_shift_flames(up, car.throttle),
            false => up && car.throttle >= trigger::MOD_SHIFT_LOAD,
        };
        let (shifted, pop_throttle, strength) = match self.mod_car {
            true => (
                self.shift.note_gear_where(car.gear, flames_at),
                0.0,
                (trigger::MOD_BACKFIRE_SECONDS, trigger::MOD_BACKFIRE_INTENSITY),
            ),
            false => {
                (self.shift.note_gear(car.gear), car.throttle, (trigger::BACKFIRE_SECONDS, trigger::BACKFIRE_INTENSITY))
            }
        };
        self.shift.advance(dt, car.gear, car.velocity.length(), self.timing);
        self.backfire.pops_for(pops, pop_throttle, car.gear, strength.0);
        if forced {
            self.backfire.start();
        }
        let blowoff = blowoff_allowed(engine_level, self.engine_upgrades);
        self.intensity = flame_intensity_with(blowoff, &self.shift, &self.backfire, strength.1);
        self.backfire.advance(dt);
        self.nitrous = car.nitrous;
        let event = match forced || (pops > 0 && pop_throttle < trigger::LIFT_OFF_THROTTLE && car.gear >= 1) {
            true => Some(crate::garage::BackfireEvent::Pop),
            false => match shifted && blowoff && self.shift.active() {
                true => Some(crate::garage::BackfireEvent::Shift),
                false => None,
            },
        };
        self.enhanced.step(dt, car, event);
        if self.intensity == 0.0 && !self.nitrous && self.live == 0 {
            return;
        }
        // The burning nitrous flames blue in place of the orange blow-off and backfire.
        // A variant's effects can stand in for the game's flames (`variant.rs`): then they spawn nothing.
        let orange = if hides_orange(self.nitrous, replaced.1, car.throttle) { 0.0 } else { self.intensity };
        let blue = if self.nitrous && !replaced.0 { 1.0 } else { 0.0 };
        for (lanes, intensity) in [(&mut self.lanes, orange), (&mut self.blue, blue)] {
            for lane in lanes {
                for (emitter, pipe) in lane.emitters.iter_mut().zip(&self.pipes) {
                    emitter.set_enabled(intensity > 0.0);
                    let frame = Frame { to_world: car.to_world * *pipe, inherit_velocity: car.velocity, intensity };
                    emitter.step(dt, &frame);
                }
            }
        }
        self.live = self.live();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_replaced_pop_hides_the_orange_flame_only_where_the_replacement_plays() {
        assert!(hides_orange(true, false, 1.0), "the nitrous burns blue");
        assert!(hides_orange(false, true, 0.0), "pedal up: the variant's pop plays in its place");
        assert!(!hides_orange(false, true, 1.0), "pedal down: the shift blow-off and mod pops keep the game's flame");
        assert!(!hides_orange(false, false, 0.0), "nothing replaces it");
    }
}
