//! Authored car environment requests. Spec: `car-audio-environment.md`.
//! Wet requests are retained; no unverified reverb processor is substituted.

use blackbox_mixmap::{curve_db, q15_from_db};

const FULL: i32 = 32767;
const TICK: f64 = 1.0 / 60.0;
const MAX_TICKS: usize = 240;

/// Inputs supplied by an authored spatial-zone adapter, when available.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnvironmentContext {
    pub zone: Option<u8>,
    /// Hundredths of a decibel, applied to wet targets only.
    pub wet_offset_db: i32,
    pub tunnel: bool,
    pub overpass: bool,
    pub garage: bool,
    pub occluded: bool,
}

impl EnvironmentContext {
    fn selected_zone(self) -> Option<u8> {
        if self.tunnel {
            return Some(5);
        }
        if self.overpass {
            return Some(6);
        }
        if self.garage {
            return Some(3);
        }
        self.zone.map(|zone| if zone < 12 { zone } else { 9 })
    }

    /// Player tunnel-controller inputs0..6, before the dynamic mixer update.
    pub fn mixer_inputs(self, levels: EnvironmentValues) -> [i32; 7] {
        let flag = |value: bool| i32::from(value) * FULL;
        let enclosed = self.tunnel || self.overpass || self.garage;
        [
            flag(enclosed && !self.overpass && !self.garage),
            flag(enclosed && self.overpass && !self.garage),
            flag(enclosed && self.garage),
            flag(enclosed),
            flag(self.occluded),
            levels.aems_dry,
            levels.aems_wet,
        ]
    }
}

/// Q15 producer values; effective wet processing is a separate capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvironmentValues {
    pub aems_dry: i32,
    pub aems_wet: i32,
    pub ginsu_dry: i32,
    pub ginsu_wet: i32,
    pub requested_patch: Option<u8>,
    pub patch_changed: bool,
    pub wet_supported: bool,
}

impl Default for EnvironmentValues {
    fn default() -> Self {
        Self {
            aems_dry: FULL,
            aems_wet: 0,
            ginsu_dry: FULL,
            ginsu_wet: 0,
            requested_patch: None,
            patch_changed: false,
            wet_supported: false,
        }
    }
}

/// Ginsu wet/dry, AEMS wet/dry in hundredths dB, fade-out/in milliseconds.
const TARGETS: [[i32; 6]; 12] = [
    [-300, 0, -300, 0, 120, 120],
    [-300, 0, -300, 0, 120, 120],
    [-300, 0, -300, 0, 120, 120],
    [-300, 0, -300, 0, 120, 120],
    [-300, 0, -300, 0, 120, 120],
    [0, -200, 0, -200, 300, 120],
    [0, -200, 0, -200, 120, 120],
    [-900, 0, -900, 0, 120, 120],
    [-1200, 0, -1200, 0, 120, 120],
    [-900, 0, -900, 0, 120, 120],
    [-900, 0, -900, 0, 120, 120],
    [-300, 0, -300, 0, 120, 120],
];

const NEUTRAL: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Steady,
    FadeOut,
    FadeIn,
}

/// Original environment transition state, clocked at60Hz with retained debt.
#[derive(Debug, Clone)]
pub struct EnvironmentController {
    debt: f64,
    current_zone: u8,
    selected_zone: Option<u8>,
    wet_offset_db: i32,
    phase: Phase,
    elapsed: f64,
    duration: f64,
    current: [f32; 4],
    target: [f32; 4],
    alternate: bool,
    values: EnvironmentValues,
}

impl Default for EnvironmentController {
    fn default() -> Self {
        Self {
            debt: 0.0,
            // The original controller starts on this row with neutral gains.
            current_zone: 5,
            selected_zone: None,
            wet_offset_db: 0,
            phase: Phase::Steady,
            elapsed: 0.0,
            duration: 0.0,
            current: NEUTRAL,
            target: NEUTRAL,
            alternate: false,
            values: EnvironmentValues::default(),
        }
    }
}

impl EnvironmentController {
    #[cfg(test)]
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub fn values(&self) -> EnvironmentValues {
        self.values
    }

    pub fn update(&mut self, dt: f32, context: EnvironmentContext) -> EnvironmentValues {
        self.values.patch_changed = false;
        if dt.is_finite() && dt > 0.0 {
            self.debt += f64::from(dt);
        }
        let mut ticks = 0;
        while self.debt + 1e-9 >= TICK && ticks < MAX_TICKS {
            self.debt = (self.debt - TICK).max(0.0);
            self.tick(context);
            ticks += 1;
        }
        if context.selected_zone().is_none() {
            self.unavailable();
        }
        self.values
    }

    fn unavailable(&mut self) {
        self.current_zone = 5;
        self.selected_zone = None;
        self.phase = Phase::Steady;
        self.current = NEUTRAL;
        self.target = NEUTRAL;
        self.alternate = false;
        self.values = EnvironmentValues::default();
    }

    fn tick(&mut self, context: EnvironmentContext) {
        let Some(zone) = context.selected_zone() else {
            self.unavailable();
            return;
        };
        if self.selected_zone != Some(zone) {
            self.selected_zone = Some(zone);
            self.wet_offset_db = context.wet_offset_db;
            self.phase = Phase::FadeOut;
            self.elapsed = 0.0;
            self.duration = f64::from(TARGETS[usize::from(self.current_zone)][4]) * 0.001;
            self.target = NEUTRAL;
        }
        if self.phase == Phase::Steady {
            self.wet_offset_db = context.wet_offset_db;
            self.target = targets(self.current_zone, self.wet_offset_db);
        }
        self.elapsed += TICK;
        if self.phase == Phase::FadeOut && self.elapsed + 1e-9 >= self.duration {
            self.current = self.target;
            self.current_zone = zone;
            self.target = targets(zone, self.wet_offset_db);
            self.phase = Phase::FadeIn;
            self.elapsed = 0.0;
            self.duration = f64::from(TARGETS[usize::from(zone)][5]) * 0.001;
            self.values.requested_patch = Some(zone);
            self.values.patch_changed = true;
        }
        if self.phase == Phase::FadeIn && self.elapsed + 1e-9 >= self.duration {
            self.current = self.target;
            self.phase = Phase::Steady;
        }
        let gains = match self.phase {
            Phase::Steady => {
                for (current, target) in self.current.iter_mut().zip(self.target) {
                    *current += (target - *current).clamp(-0.25, 0.25);
                }
                self.current
            }
            Phase::FadeOut | Phase::FadeIn => {
                let fraction = (self.elapsed / self.duration).clamp(0.0, 1.0) as f32;
                std::array::from_fn(|i| self.current[i] + (self.target[i] - self.current[i]) * fraction)
            }
        };
        let q15 = |gain: f32| (gain * FULL as f32) as i32;
        self.values.ginsu_wet = q15(gains[0]);
        self.values.ginsu_dry = q15(gains[1]);
        self.values.aems_wet = q15(gains[2]);
        self.values.aems_dry = q15(gains[3]);
        if self.alternate {
            self.values.ginsu_wet = (self.values.ginsu_wet + 1).min(FULL);
            self.values.aems_wet = (self.values.aems_wet + 1).min(FULL);
        }
        self.alternate = !self.alternate;
    }
}

fn targets(zone: u8, offset_db: i32) -> [f32; 4] {
    let row = TARGETS[usize::from(zone)];
    std::array::from_fn(|i| {
        let level = match i {
            0 | 2 => row[i].saturating_add(offset_db).clamp(-10000, 0),
            _ => row[i],
        };
        q15_from_db(level) as f32 / FULL as f32
    })
}

/// Camera-distance cutoff. The shared mathematical dB table differs from the
/// original by small Q15 units; the original-vector regression bounds that gap.
pub fn distance_filter(camera_distance_cm: i32) -> i32 {
    let distance = camera_distance_cm.max(0) as f32 * 0.01;
    let factor = ((distance - 6.5) / 44.0).clamp(0.0, 1.0);
    let input = (factor * 32766.0) as i32;
    let gain = q15_from_db(curve_db(6, input));
    (gain as f32 / FULL as f32 * 23275.0 + 725.0) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(zone: u8) -> EnvironmentContext {
        EnvironmentContext { zone: Some(zone), ..EnvironmentContext::default() }
    }

    #[test]
    fn unavailable_environment_never_invents_wet_or_a_patch() {
        let mut controller = EnvironmentController::new();
        assert_eq!(controller.update(1.0, EnvironmentContext::default()), EnvironmentValues::default());
        controller.update(1.0, context(5));
        assert_eq!(controller.update(0.0, EnvironmentContext::default()), EnvironmentValues::default());
    }

    #[test]
    fn patch_switch_waits_for_fade_out_and_wet_fades_in_afterward() {
        let mut controller = EnvironmentController::new();
        let early = controller.update(0.25, context(9));
        assert_eq!(early.requested_patch, None);
        assert!(early.aems_wet <= 1);
        let switch = controller.update(0.05, context(9));
        assert_eq!(switch.requested_patch, Some(9));
        assert!(switch.patch_changed && !switch.wet_supported);
        assert!(switch.aems_wet <= 1);
        let halfway = controller.update(0.05, context(9));
        let late = controller.update(0.2, context(9));
        assert!(halfway.aems_wet > 1 && halfway.aems_wet < late.aems_wet);
        assert_eq!(late.aems_dry, FULL);
    }

    #[test]
    fn wet_offset_preserves_dry_and_fades_keep_the_selected_offset() {
        let mut controller = EnvironmentController::new();
        let tunnel = EnvironmentContext { tunnel: true, ..context(9) };
        let initial = controller.update(1.0, tunnel);
        assert_eq!(initial.aems_wet, FULL);
        assert!(initial.aems_dry < FULL && initial.aems_dry > 25000);
        let quiet = controller.update(1.0, EnvironmentContext { wet_offset_db: -10000, ..tunnel });
        assert_eq!(quiet.aems_dry, initial.aems_dry);
        assert!(quiet.aems_wet <= 1);
    }

    #[test]
    fn split_updates_and_retained_debt_produce_the_same_controller_state() {
        let mut bulk = EnvironmentController::new();
        let mut split = EnvironmentController::new();
        bulk.update(1.0, context(8));
        for _ in 0..120 {
            split.update(1.0 / 120.0, context(8));
        }
        let mut bulk_state = bulk.values();
        assert!(bulk_state.patch_changed, "the bulk interval contains a patch-switch event");
        bulk_state.patch_changed = false;
        assert_eq!(bulk_state, split.values());
        let mut delayed = EnvironmentController::new();
        delayed.update(8.0, context(8));
        delayed.update(0.0, context(8));
        let mut all = EnvironmentController::new();
        for _ in 0..480 {
            all.update(1.0 / 60.0, context(8));
        }
        assert_eq!(delayed.values(), all.values());
    }

    #[test]
    fn camera_distance_vectors_bound_the_shared_conversion_approximation() {
        for (cm, original) in [
            (0, 23920),
            (560, 23920),
            (650, 23920),
            (1000, 18605),
            (2000, 7398),
            (3000, 2257),
            (4000, 837),
            (5050, 725),
            (10000, 725),
        ] {
            let result = distance_filter(cm);
            assert!((result - original).abs() <= 3, "{cm}cm: {result}Hz, original {original}Hz");
        }
        assert_eq!(distance_filter(i32::MAX), 725);
        assert_eq!(distance_filter(-1), distance_filter(0));
    }

    #[test]
    fn authored_context_publishes_the_original_controller_inputs() {
        let levels = EnvironmentValues { aems_dry: 12345, aems_wet: 6789, ..EnvironmentValues::default() };
        let garage = EnvironmentContext { garage: true, occluded: true, ..context(9) };
        assert_eq!(garage.mixer_inputs(levels), [0, 0, FULL, FULL, FULL, 12345, 6789]);
        let open = context(9);
        assert_eq!(open.mixer_inputs(levels), [0, 0, 0, 0, 0, 12345, 6789]);
    }
}
