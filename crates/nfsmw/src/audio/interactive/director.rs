//! The decisions: which music plays for a game state, at which control value, and when the songs may return.
//!
//! Pure logic with no sound device, so the rules are tested directly. The numbers marked **[guess]** are this
//! project's; the original's values live in event actions that are not interpreted (spec section 8).

use super::state::MusicState;

/// Seconds the licensed songs stay off after a pursuit ends. The spec reads 40 s in the original **[inferred]**.
pub const RESUME_DELAY: f32 = 40.0;
/// Seconds a new pursuit set must be asked for before the music switches to it, so a heat level that flickers
/// does not make the music flip back and forth **[guess]**.
pub const SET_HOLD: f32 = 2.0;
/// How fast the control value follows a rising intensity, in control units per second **[guess]**.
pub const RISE: f32 = 40.0;
/// How fast it follows a falling intensity; slower, so a short lull does not drop the music **[guess]**.
pub const FALL: f32 = 8.0;

/// What the music should do this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    /// The pursuit set to play, if any.
    pub pursuit: Option<u8>,
    /// The graph's control value, 0 to 127. Only meaningful with a pursuit set.
    pub control: u8,
    /// Whether the licensed songs may play.
    pub radio: bool,
}

/// The control value for an intensity of 0 to 1.
pub fn control_value(intensity: f32) -> u8 {
    (intensity.clamp(0.0, 1.0) * 127.0).round() as u8
}

#[derive(Debug, Default)]
pub struct Director {
    /// The set being played.
    active: Option<u8>,
    /// A different set that is asked for, and for how long.
    candidate: Option<(u8, f32)>,
    /// The smoothed control value.
    control: f32,
    /// Seconds before the songs may return after a pursuit.
    resume: Option<f32>,
}

impl Director {
    /// Seconds left before the licensed songs may play again, if a pursuit just ended.
    pub fn resume_in(&self) -> Option<f32> {
        self.resume
    }

    pub fn control(&self) -> u8 {
        self.control.round().clamp(0.0, 127.0) as u8
    }

    /// One frame. `driving` is true while a game is on (driving or paused); outside it nothing is held back and
    /// the pursuit music is dropped.
    pub fn update(&mut self, state: &MusicState, driving: bool, dt: f32) -> Plan {
        let dt = dt.max(0.0);
        let Some(pursuit) = state.pursuit.filter(|_| driving) else {
            self.pursuit_over(driving, dt);
            return Plan { pursuit: None, control: self.control(), radio: self.resume.is_none() };
        };
        self.resume = None;
        let target = f32::from(control_value(pursuit.intensity));
        let Some(active) = self.active else {
            self.active = Some(pursuit.set);
            self.candidate = None;
            self.control = target;
            return self.plan();
        };
        self.follow(target, dt);
        if pursuit.set == active {
            self.candidate = None;
            return self.plan();
        }
        let held = match self.candidate {
            Some((set, held)) if set == pursuit.set => held + dt,
            _ => 0.0,
        };
        self.candidate = Some((pursuit.set, held));
        if held >= SET_HOLD {
            self.active = Some(pursuit.set);
            self.candidate = None;
        }
        self.plan()
    }

    fn plan(&self) -> Plan {
        Plan { pursuit: self.active, control: self.control(), radio: false }
    }

    /// No pursuit this frame: start the delay when one just ended, count it down, and forget everything when
    /// the game is left.
    fn pursuit_over(&mut self, driving: bool, dt: f32) {
        self.candidate = None;
        if !driving {
            self.active = None;
            self.resume = None;
            return;
        }
        if self.active.take().is_some() {
            self.resume = Some(RESUME_DELAY);
            return;
        }
        let Some(left) = self.resume else { return };
        self.resume = Some(left - dt).filter(|&left| left > 0.0);
    }

    fn follow(&mut self, target: f32, dt: f32) {
        if target > self.control {
            self.control = (self.control + RISE * dt).min(target);
            return;
        }
        self.control = (self.control - FALL * dt).max(target);
    }
}
