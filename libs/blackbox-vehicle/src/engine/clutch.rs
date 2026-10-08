use crate::math::ramp;

/// Clutch states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClutchState {
    Engaged,
    /// Re-engaging: slips until the timer runs out.
    Engaging,
    Disengaged,
}

/// The clutch: a three-state machine with an engage timer.
#[derive(Clone, Copy, Debug)]
pub struct Clutch {
    pub state: ClutchState,
    timer: f32,
    engage_time: f32,
}

impl Default for Clutch {
    fn default() -> Self {
        Self { state: ClutchState::Engaged, timer: 0.0, engage_time: 0.0 }
    }
}

impl Clutch {
    /// Grip factor: 1 engaged, 0.25 disengaged, in between while engaging.
    pub fn factor(&self) -> f32 {
        match self.state {
            ClutchState::Engaged => 1.0,
            ClutchState::Engaging => 1.0 - 0.75 * ramp(self.timer, 0.0, self.engage_time),
            ClutchState::Disengaged => 0.25,
        }
    }

    pub fn is_engaged(&self) -> bool {
        self.state == ClutchState::Engaged
    }

    /// Only acts from `Engaged`.
    pub fn disengage(&mut self) {
        if self.state == ClutchState::Engaged {
            self.state = ClutchState::Disengaged;
        }
    }

    /// Only acts from `Disengaged`; engages at once when `time <= 0`.
    pub fn engage(&mut self, time: f32) {
        if self.state == ClutchState::Disengaged {
            if time <= 0.0 {
                self.state = ClutchState::Engaged;
            } else {
                self.state = ClutchState::Engaging;
                self.timer = time;
                self.engage_time = time;
            }
        }
    }

    /// Counts the engage timer down.
    pub fn update(&mut self, dt: f32) {
        if self.state == ClutchState::Engaging {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.timer = 0.0;
                self.state = ClutchState::Engaged;
            }
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engage_cycle() {
        let mut c = Clutch::default();
        assert_eq!(c.factor(), 1.0);
        c.engage(0.25); // no effect while engaged
        assert!(c.is_engaged());
        c.disengage();
        assert_eq!(c.factor(), 0.25);
        c.disengage();
        c.engage(0.25);
        assert_eq!(c.state, ClutchState::Engaging);
        assert!((c.factor() - 0.25).abs() < 1e-6);
        c.update(0.125);
        assert!((c.factor() - 0.625).abs() < 1e-6);
        c.update(0.2);
        assert!(c.is_engaged());
    }

    #[test]
    fn zero_time_engages_at_once() {
        let mut c = Clutch::default();
        c.disengage();
        c.engage(0.0);
        assert!(c.is_engaged());
    }
}
