//! Inputs for the road action's nitrous decision (ai-pursuit-road-nitrous.md).

#[cfg(test)]
#[path = "road_nos_rules_tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::scenes::world::ai) struct Input {
    /// Extra torque multiplier, excluding the unboosted one.
    pub boost: f32,
    /// Seconds of full burn in this vehicle's own bottle.
    pub bottle_seconds: f32,
    /// Remaining tank fraction.
    pub capacity: f32,
    /// Previous effective held nitrous control.
    pub was_on: bool,
}

impl Input {
    fn matched_boost(self, target_boost: f32) -> f32 {
        self.boost.min(target_boost).max(0.0)
    }

    pub(super) fn acceleration_boost(self, target_boost: f32) -> Option<f32> {
        (self.was_on && self.capacity > 0.0).then_some(self.matched_boost(target_boost))
    }
}

pub(super) struct Decision {
    pub speed: f32,
    pub requested_speed: f32,
    pub potential_speed: f32,
    pub potential_acceleration: f32,
    pub actual_acceleration: f32,
    pub occluded: bool,
    pub stopping: bool,
    pub skill: f32,
}

#[derive(Debug, Clone, Default)]
pub(super) struct State {
    active: bool,
    requested: bool,
    timer: f32,
}

impl State {
    pub(super) fn active(&self) -> bool {
        self.active
    }

    pub(super) fn requested(&self) -> bool {
        self.requested
    }

    pub(super) fn begin(&mut self) {
        *self = Self { active: true, ..Default::default() };
    }

    pub(super) fn leave(&mut self) {
        *self = Self::default();
    }

    pub(super) fn hash_into(&self, hash: &mut crate::scenes::world::StateHasher) {
        hash.bool(self.active);
        hash.bool(self.requested);
        hash.f32(self.timer);
    }

    pub(super) fn update(&mut self, input: Input, target_boost: f32, decision: Decision, dt: f32) {
        self.requested = self.choose(input, target_boost, &decision);
        self.timer = match self.requested {
            true => (self.timer + dt).max(0.0),
            false => (self.timer - dt).min(0.0),
        };
    }

    fn choose(&self, input: Input, target_boost: f32, decision: &Decision) -> bool {
        if decision.stopping
            || decision.speed < 10.0
            || decision.speed >= decision.requested_speed
            || input.matched_boost(target_boost) <= 0.0
            || input.bottle_seconds <= 0.0
        {
            return false;
        }
        let was_on = input.was_on && input.capacity > 0.0;
        let availability = 0.33 + (1.0 - 0.33) * decision.skill;
        let threshold = match was_on {
            true => 1.0 - availability,
            false => availability * (0.5 + (0.3 - 0.5) * decision.skill),
        };
        let available_time = (input.capacity - threshold) * input.bottle_seconds;
        if input.capacity <= threshold || available_time <= 0.0 {
            return false;
        }
        let scale = 0.25 + 0.75 * decision.skill;
        let speed_gap = scale * (decision.potential_speed - decision.speed);
        let acceleration_gap = scale * (decision.potential_acceleration - decision.actual_acceleration);
        if was_on {
            let minimum_burn = 2.0 + 2.0 * decision.skill;
            return !(self.timer > minimum_burn
                && (speed_gap < 5.0 || acceleration_gap < 0.15 * decision.potential_acceleration));
        }
        let minimum_rest = 20.0 - 16.0 * decision.skill;
        self.timer < -minimum_rest
            && !decision.occluded
            && speed_gap > 15.0
            && acceleration_gap > 0.5 * decision.potential_acceleration
    }
}
