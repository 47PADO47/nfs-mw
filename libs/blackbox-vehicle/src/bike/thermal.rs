//! The temperature and the wear of a tyre, and the efficiency they give (`docs/specs/ride6-tyre-thermal.md`).
//!
//! Each strip of the tread has a temperature and a thickness and takes a share of the contact patch (tent weights across the
//! width). Heat comes from sliding and rolling, cooling goes towards the road and air temperature, and the tread wears with the
//! distance slid or rolled. The efficiency is the weighted mean of what each strip's temperature and wear allow.

use super::choices;
use super::curve::factor;
use super::spec::{TyreSection, TyreThermal};

/// What the tyre did this step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Work {
    /// The load (N); 0 for a wheel in the air.
    pub load: f32,
    /// The longitudinal and lateral slip normalised to the force peaks (their absolute values are used, at most 10).
    pub normalised_x: f32,
    pub normalised_y: f32,
    /// The rolling resistance force (N).
    pub rolling: f32,
    /// The wheel's lean from the ground normal (rad).
    pub camber: f32,
    /// The slip speed and the speed of the contact point (m/s).
    pub slip_speed: f32,
    pub contact_speed: f32,
    pub spin: f32,
    pub in_contact: bool,
}

/// The state of one tyre.
#[derive(Clone, Debug)]
pub struct TyreState {
    temperature: Vec<f32>,
    thickness: Vec<f32>,
    weights: Vec<f32>,
    efficiency: f32,
}

/// The share of the contact patch on each of `n` strips for a wheel leaned by `camber` (tent weights over the span).
pub fn section_weights(count: usize, dependency: f32, span: f32, camber: f32) -> Vec<f32> {
    if count <= 1 {
        return vec![1.0; count];
    }
    let step = 2.0 * span / (count - 1) as f32;
    let width = step * dependency;
    let lean = camber.clamp(-(span + 0.9999 * width), span + 0.9999 * width);
    let mut weights: Vec<f32> = (0..count)
        .map(|index| {
            let centre = -span + index as f32 * step;
            (1.0 - (centre - lean).abs() / width.max(1e-6)).max(0.0)
        })
        .collect();
    let sum: f32 = weights.iter().sum();
    if sum > 0.0 {
        weights.iter_mut().for_each(|w| *w /= sum);
    }
    weights
}

/// What one strip's temperature and wear allow.
fn section_efficiency(section: &TyreSection, temperature: f32, worn: f32) -> f32 {
    let value = factor(&section.temperature_to_efficiency, temperature) * factor(&section.wear_to_efficiency, worn);
    value.clamp(section.efficiency_limit, 1.0)
}

impl TyreState {
    /// A new tyre at its starting temperature (the floor), or one without a thermal model.
    pub fn new(spec: Option<&TyreThermal>) -> Self {
        let Some(spec) = spec else {
            return Self { temperature: Vec::new(), thickness: Vec::new(), weights: Vec::new(), efficiency: 1.0 };
        };
        let count = spec.sections.len();
        let mut state = Self {
            temperature: vec![spec.min_temperature; count],
            thickness: vec![spec.initial_thickness; count],
            weights: vec![1.0 / count.max(1) as f32; count],
            efficiency: 1.0,
        };
        state.efficiency = state.mix(spec);
        state
    }

    /// The efficiency of the tyre now, 0 to 1.
    pub fn efficiency(&self) -> f32 {
        self.efficiency
    }

    /// The mean temperature of the strips (deg C), 0 for a tyre without a model.
    pub fn temperature(&self) -> f32 {
        match self.temperature.is_empty() {
            true => 0.0,
            false => self.temperature.iter().sum::<f32>() / self.temperature.len() as f32,
        }
    }

    /// The mean worn thickness of the tread (units of the tyre data).
    pub fn wear(&self, spec: &TyreThermal) -> f32 {
        match self.thickness.is_empty() {
            true => 0.0,
            false => spec.initial_thickness - self.thickness.iter().sum::<f32>() / self.thickness.len() as f32,
        }
    }

    fn mix(&self, spec: &TyreThermal) -> f32 {
        let worn = |index: usize| spec.initial_thickness - self.thickness[index];
        let sum: f32 = spec
            .sections
            .iter()
            .enumerate()
            .map(|(index, section)| {
                self.weights[index] * section_efficiency(section, self.temperature[index], worn(index))
            })
            .sum();
        sum.clamp(0.0, 1.0)
    }

    /// Advance the tyre by `dt`: the weights across the tread, the wear (in contact), the temperature, the transfer between
    /// strips, the uniform mean, and the efficiency.
    pub fn step(&mut self, spec: &TyreThermal, work: &Work, dt: f32) {
        let count = spec.sections.len();
        if count == 0 || self.temperature.len() != count || dt <= 0.0 {
            return;
        }
        self.weights = section_weights(count, spec.sections_dependency, spec.sections_angle, work.camber);
        if work.in_contact {
            self.wear_step(spec, work, dt);
        }
        self.temperature_step(spec, work, dt);
        if spec.heat_transfer > 0.0 && count > 1 {
            self.transfer(spec.heat_transfer, dt);
        }
        if spec.uniform {
            let mean_temperature = self.temperature.iter().sum::<f32>() / count as f32;
            let mean_thickness = self.thickness.iter().sum::<f32>() / count as f32;
            self.temperature.iter_mut().for_each(|t| *t = mean_temperature);
            self.thickness.iter_mut().for_each(|h| *h = mean_thickness);
        }
        self.efficiency = self.mix(spec);
    }

    fn wear_step(&mut self, spec: &TyreThermal, work: &Work, dt: f32) {
        let speed = work.slip_speed.abs().max(work.contact_speed.abs());
        let length = (spec.wear_length * spec.decay_distance_factor).max(1e-6);
        for (index, section) in spec.sections.iter().enumerate() {
            let rate = factor(&section.temperature_to_wear, self.temperature[index]);
            let kept = match spec.decay_distance_factor == 0.0 {
                true => 0.0,
                false => rate.max(0.0).powf(speed * dt / length),
            };
            let h = &mut self.thickness[index];
            *h *= 1.0 + spec.wear_intensity * self.weights[index] * (kept - 1.0);
        }
    }

    fn temperature_step(&mut self, spec: &TyreThermal, work: &Work, dt: f32) {
        let air = choices::AIR_TEMPERATURE;
        let road = choices::ASPHALT_TEMPERATURE;
        let steady = (1.0 - spec.asphalt_importance) * air + spec.asphalt_importance * road;
        let backwards = work.spin < 0.0;
        let (nx, ny, rolling) = match backwards {
            true => (0.0, 0.0, 0.0),
            false => (work.normalised_x.abs().min(10.0), work.normalised_y.abs().min(10.0), work.rolling * 100.0),
        };
        // The coefficients are in Unreal force units: a newton is a hundred of them.
        let load = work.load * 100.0;
        for (index, section) in spec.sections.iter().enumerate() {
            let heat_work =
                load * (section.lat_slip_coef * ny + section.long_slip_coef * nx) + section.rolling_coef * rolling;
            let heat = (-heat_work * section.dynamic_coef * dt * spec.dynamic_time_scale).exp();
            let cool = (-section.static_coef * dt * spec.static_time_scale).exp();
            let t = self.temperature[index];
            let next =
                t + (1.0 - heat) * self.weights[index] * (spec.max_temperature - t) + (1.0 - cool) * (steady - t);
            self.temperature[index] = next.clamp(spec.min_temperature, spec.max_temperature);
        }
    }

    /// Neighbouring strips relax to one temperature at a square-root rate (`TireSectionsHeatTransfer`).
    fn transfer(&mut self, rate: f32, dt: f32) {
        for index in 1..self.temperature.len() {
            let mut moved = (self.temperature[index] - self.temperature[index - 1]) / 2.0;
            let excess = moved.abs().sqrt() - dt * rate;
            if excess > 0.0 {
                moved -= moved.signum() * excess * excess;
            }
            self.temperature[index - 1] += moved;
            self.temperature[index] -= moved;
        }
    }
}

/// The wear rate of a strip at a temperature, for the tests of the curves.
#[cfg(test)]
fn rate_at(section: &TyreSection, temperature: f32) -> f32 {
    factor(&section.temperature_to_wear, temperature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    fn spec() -> TyreThermal {
        BikeSpec::example().front.thermal.unwrap()
    }

    #[test]
    fn the_weights_follow_the_lean_across_three_strips() {
        let upright = section_weights(3, 1.0, 1.0, 0.0);
        assert!((upright[1] - 1.0).abs() < 1e-6 && upright[0] == 0.0);
        let leaned = section_weights(3, 1.0, 1.0, 0.5);
        assert!((leaned[1] - 0.5).abs() < 1e-6 && (leaned[2] - 0.5).abs() < 1e-6);
        assert!((leaned.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert_eq!(section_weights(1, 1.0, 1.0, 0.3), vec![1.0]);
    }

    #[test]
    fn a_new_tyre_is_cold_and_gives_the_cold_efficiency() {
        let spec = spec();
        let state = TyreState::new(Some(&spec));
        assert!((state.efficiency() - 0.975).abs() < 0.02, "{}", state.efficiency());
        assert_eq!(TyreState::new(None).efficiency(), 1.0);
    }

    #[test]
    fn sliding_and_rolling_warm_the_tyre_and_it_cools_towards_the_road() {
        let spec = spec();
        let mut state = TyreState::new(Some(&spec));
        let work = Work {
            load: 2000.0,
            normalised_x: 1.0,
            normalised_y: 1.0,
            rolling: 60.0,
            in_contact: true,
            spin: 50.0,
            ..Work::default()
        };
        for _ in 0..600 {
            state.step(&spec, &work, 1.0 / 60.0);
        }
        let warmed = state.temperature();
        assert!(warmed > 0.0, "{warmed}");
        let idle = Work::default();
        for _ in 0..600 {
            state.step(&spec, &idle, 1.0 / 60.0);
        }
        assert!(state.temperature() <= warmed + 1e-3);
    }

    #[test]
    fn the_tread_wears_with_the_distance_and_grip_falls_with_the_wear() {
        let spec = spec();
        let mut state = TyreState::new(Some(&spec));
        let work = Work { load: 2000.0, contact_speed: 50.0, in_contact: true, spin: 50.0, ..Work::default() };
        let before = state.wear(&spec);
        for _ in 0..600 {
            state.step(&spec, &work, 1.0 / 60.0);
        }
        assert!(state.wear(&spec) > before);
    }

    #[test]
    fn a_warm_tyre_grips_better_than_a_cold_one() {
        let spec = spec();
        let mut state = TyreState::new(Some(&spec));
        state.temperature.iter_mut().for_each(|t| *t = 95.0);
        state.step(&spec, &Work::default(), 1.0 / 60.0);
        assert!(state.efficiency() > 0.98, "{}", state.efficiency());
        assert!(rate_at(&spec.sections[0], 88.0) > 0.9);
    }

    #[test]
    fn the_strips_relax_to_one_temperature_when_heat_moves_between_them() {
        let mut spec = spec();
        spec.uniform = false;
        spec.heat_transfer = 5.0;
        let mut state = TyreState::new(Some(&spec));
        state.temperature = vec![100.0, 0.0, 0.0];
        for _ in 0..600 {
            state.transfer(spec.heat_transfer, 1.0 / 60.0);
        }
        assert!(state.temperature[0] < 100.0 && state.temperature[1] > 0.0);
    }
}
