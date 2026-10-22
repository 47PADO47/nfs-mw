//! Destroyed NPC sleep eligibility; `docs/specs/wreck-motion.md`.

use super::Vehicle;

#[derive(Clone, Debug, Default)]
pub(super) struct Retirement {
    delay: Option<f32>,
    supported_time: f32,
    pending: bool,
    object_collision: bool,
}

impl Vehicle {
    /// Caller-selected support duration before ordinary body sleep is allowed. `None` disables it.
    /// This does not shape controls or replace the body's motion/contact thresholds.
    pub fn set_retirement_sleep_delay(&mut self, delay: Option<f32>) {
        let delay = delay.filter(|delay| delay.is_finite() && *delay >= 0.0);
        if self.retirement.delay != delay {
            self.retirement.supported_time = 0.0;
        }
        self.retirement.delay = delay;
    }

    pub(super) fn begin_retirement_frame(&mut self) {
        self.retirement.pending = false;
        self.retirement.object_collision = false;
    }

    pub(super) fn advance_retirement_support(&mut self, dt: f32) {
        self.retirement.supported_time = match self.retirement.delay.is_some() && self.wheels_on_ground > 2 {
            true => self.retirement.supported_time + dt,
            false => 0.0,
        };
        self.retirement.pending = true;
    }

    /// Keeps ordinary collision notification and records the distinct native object-hit sleep veto.
    pub fn notify_object_collision(&mut self, impulse: f32) {
        self.notify_collision(impulse);
        self.mark_object_contact();
    }

    /// An admitted overlap sets the native contact flag even without an approaching response.
    /// This status change does not itself apply force or wake a sleeping body.
    pub fn mark_object_contact(&mut self) {
        self.retirement.object_collision = true;
    }

    /// Finish once after external object/world contacts; repeated pose sync does not advance time.
    pub fn finish_retirement_contacts(&mut self) {
        let Some(delay) = self.retirement.delay else { return };
        if !std::mem::take(&mut self.retirement.pending)
            || self.retirement.supported_time < delay
            || self.retirement.object_collision
        {
            return;
        }
        self.body.update_sleep(self.wheels_on_ground + self.body_contacts.len());
    }
}

#[cfg(test)]
mod tests;
