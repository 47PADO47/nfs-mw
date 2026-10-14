//! Voice gain targets and the matched sixteen-sample transition.
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
pub(super) struct Params {
    pub gain: f32,
    pub dry: f32,
    pub wet: f32,
    pub pan: f32,
    pub mono: bool,
}

impl Params {
    pub fn gains(self) -> [f32; 3] {
        let dry = self.gain * self.dry;
        if !self.mono {
            return [dry, dry, self.gain * self.wet];
        }
        let pan = self.pan.clamp(-1.0, 1.0);
        [dry * ((1.0 - pan) * 0.5).sqrt(), dry * ((1.0 + pan) * 0.5).sqrt(), self.gain * self.wet]
    }
}

#[derive(Clone)]
pub(super) struct Control(Arc<Mutex<Params>>);

impl Control {
    pub fn from_params(params: Params) -> Self {
        Self(Arc::new(Mutex::new(params)))
    }
    pub fn set(&self, change: impl FnOnce(&mut Params)) {
        if let Ok(mut params) = self.0.lock() {
            change(&mut params);
        }
    }
    pub fn snapshot(&self) -> Option<Params> {
        self.0.try_lock().ok().map(|params| *params)
    }
}

pub(super) struct Ramp {
    current: [f32; 3],
    target: [f32; 3],
    delta: [f32; 3],
    left: u8,
}

impl Ramp {
    pub fn new(gains: [f32; 3]) -> Self {
        Self { current: gains, target: gains, delta: [0.0; 3], left: 0 }
    }
    pub fn set(&mut self, gains: [f32; 3]) {
        if self.target == gains {
            return;
        }
        self.target = gains;
        self.delta = std::array::from_fn(|i| (gains[i] - self.current[i]) / 16.0);
        self.left = 16;
    }
    pub fn next(&mut self) -> [f32; 3] {
        if self.left == 0 {
            return self.current;
        }
        self.left -= 1;
        for i in 0..3 {
            self.current[i] += self.delta[i];
        }
        if self.left == 0 {
            self.current = self.target;
        }
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wet_gain_is_independent_of_dry_and_pan() {
        let params = Params { gain: 0.5, dry: 0.0, wet: 0.75, pan: -1.0, mono: true };
        assert_eq!(params.gains(), [0.0, 0.0, 0.375]);
        assert_eq!(Params { dry: 1.0, pan: 1.0, ..params }.gains(), [0.0, 0.5, 0.375]);
    }
    #[test]
    fn first_sample_moves_and_sixteenth_reaches_target_without_render_time() {
        let mut ramp = Ramp::new([0.0; 3]);
        ramp.set([1.0, 0.5, 0.25]);
        assert_eq!(ramp.next(), [1.0 / 16.0, 0.5 / 16.0, 0.25 / 16.0]);
        for _ in 1..16 {
            ramp.next();
        }
        assert_eq!(ramp.next(), [1.0, 0.5, 0.25]);
        ramp.set([1.0, 0.5, 0.25]);
        assert_eq!(ramp.next(), [1.0, 0.5, 0.25]);
    }
}
