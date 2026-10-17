//! The cop health bars on the traffic side: the samples of the cop cars, and the bars of one simulated step. The rules
//! are in `blackbox_pursuit::health_bar`; the bar state is kept by the scene (`effects/cop_health.rs`). Spec:
//! `docs/specs/cop-health-bars.md` section 13.2, steps 2 and 3.

use blackbox_pursuit::health_bar::{Bar, BarParams, CopBarStates, Frame, Gates, Sample, build_bars};

use super::TrafficWorld;

impl TrafficWorld {
    /// The health sample of every cop car, wrecked ones included, appended to `out`. Helicopters are not in `cars`,
    /// and roadblock cars are `Role::Traffic`, so neither gets a sample.
    pub fn cop_health_samples(&self, out: &mut Vec<Sample>) {
        out.extend(self.cars.iter().filter_map(|car| car.cop_health_sample()));
    }

    /// Steps the bar states of every cop by `dt` simulated seconds and returns the bars to draw, back to front. The
    /// chase gate is read here; the shot and HUD gates come in `frame`.
    pub fn cop_health_bars(&self, states: &mut CopBarStates, frame: Frame, dt: f32, params: &BarParams) -> Vec<Bar> {
        let mut samples = Vec::new();
        self.cop_health_samples(&mut samples);
        let gates = Gates { pursuing: self.is_pursuing(), ..frame.gates };
        build_bars(&samples, &Frame { gates, ..frame }, states, dt, params)
    }
}
