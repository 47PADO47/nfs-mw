//! Original authored cloud targets and high-FPS-safe easing. Spec: `world-atmosphere.md`.

use blackbox_hash::bstring_hash;
use blackbox_paramap::Layer;

#[derive(Default)]
pub(in crate::scenes::world) struct Clouds {
    layer: Option<Layer>,
    intensity: f64,
    override_: Option<f32>,
}

impl Clouds {
    pub(in crate::scenes::world) fn read(metadata: &[u8]) -> anyhow::Result<Self> {
        let layer = Layer::read_all(metadata)?.into_iter().find(|layer| layer.name_hash == bstring_hash("Clouds"));
        Ok(Self { layer, intensity: 0.0, override_: None })
    }

    pub(in crate::scenes::world) fn intensity(&self) -> f32 {
        self.intensity as f32
    }

    pub(in crate::scenes::world) fn override_amount(&self) -> Option<f32> {
        self.override_
    }

    pub(in crate::scenes::world) fn has_authored_map(&self) -> bool {
        self.layer.is_some()
    }

    /// Explicit comparison selections apply while paused; automatic easing does not.
    pub(in crate::scenes::world) fn configure(
        &mut self,
        amount: Option<f32>,
        settle: bool,
        position: [f32; 2],
        rain: f32,
    ) {
        if amount.is_some_and(|value| !value.is_finite()) {
            return;
        }
        let amount = amount.map(|value| value.clamp(0.0, 1.0));
        let changed = amount != self.override_;
        self.override_ = amount;
        if settle || (changed && amount.is_some()) {
            self.intensity = f64::from(self.target(position, rain));
        }
    }

    pub(in crate::scenes::world) fn target(&self, position: [f32; 2], rain: f32) -> f32 {
        if let Some(amount) = self.override_ {
            return amount;
        }
        if rain.is_finite() && rain > 0.0 {
            return rain.clamp(0.0, 1.0);
        }
        let Some(layer) = &self.layer else { return 0.0 };
        let Some(set) = layer.set_at(position[0], position[1]) else { return 0.0 };
        let (Some(intensity), Some(threshold)) = (layer.float(set, 0), layer.float(set, 1)) else { return 0.0 };
        selected_target(intensity, threshold, free_roam_gate(position[0]))
    }

    pub(in crate::scenes::world) fn update(&mut self, dt: f32, position: [f32; 2], rain: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let target = f64::from(self.target(position, rain));
        // Original dt/10 at 60 Hz, integrated independently of render frame rate.
        let alpha = -((1.0_f64 - 1.0 / 600.0).ln() * 60.0 * f64::from(dt)).exp_m1();
        self.intensity += (target - self.intensity) * alpha;
        if target == 0.0 && self.intensity < 0.01 {
            self.intensity = 0.0;
        }
        self.intensity = self.intensity.clamp(0.0, 1.0);
    }
}

fn selected_target(intensity: f32, threshold: f32, gate: f32) -> f32 {
    if !intensity.is_finite() || !threshold.is_finite() || gate > threshold {
        return 0.0;
    }
    intensity.clamp(0.0, 1.0)
}

fn free_roam_gate(x: f32) -> f32 {
    if !x.is_finite() {
        return 1.0;
    }
    let degrees = 0.714_285_73 + x * 0.02;
    let angle = ((degrees * 65_536.0).trunc() as i64 / 360) as u16;
    let sine = (f32::from(angle) * (std::f32::consts::TAU / 65_536.0)).sin().max(0.0);
    1.0 - sine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_roam_gate_includes_position_and_suppresses_the_negative_half() {
        assert!(free_roam_gate(0.0) > 0.98);
        assert!(free_roam_gate((90.0 - 0.714_285_73) / 0.02) < 0.00001);
        assert_eq!(free_roam_gate((270.0 - 0.714_285_73) / 0.02), 1.0);
        assert_eq!(free_roam_gate(f32::NAN), 1.0);
    }

    #[test]
    fn threshold_equality_selects_authored_intensity_and_bad_fields_are_unavailable() {
        assert_eq!(selected_target(0.5, 0.25, 0.25), 0.5);
        assert_eq!(selected_target(0.5, 0.25, 0.25001), 0.0);
        assert_eq!(selected_target(f32::NAN, 0.25, 0.0), 0.0);
        assert_eq!(selected_target(0.5, f32::NAN, 0.0), 0.0);
        let missing = Clouds::default();
        assert_eq!(missing.target([0.0; 2], 0.0), 0.0);
        assert_eq!(missing.target([0.0; 2], 0.5), 0.5);
        assert_eq!(missing.target([f32::NAN; 2], 0.0), 0.0);
    }

    #[test]
    fn response_preserves_sixty_hz_and_is_independent_of_render_rate() {
        let expected = 1.0 - (1.0_f64 - 1.0 / 600.0).powi(600);
        for fps in [30, 60, 144, 240, 360] {
            let mut state = Clouds::default();
            for _ in 0..fps * 10 {
                state.update(1.0 / fps as f32, [0.0; 2], 1.0);
            }
            assert!((f64::from(state.intensity()) - expected).abs() < 0.000001, "{fps}");
            let before = state.intensity();
            for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
                state.update(dt, [0.0; 2], 1.0);
            }
            assert_eq!(state.intensity(), before);
        }
    }

    #[test]
    #[ignore = "needs the original installed track selected by NFSMW_GAME_DIR"]
    fn installed_cloud_map_has_authored_targets_and_rain_precedence() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let bytes = nfsmw_data::read_unwrapped(&dir, "TRACKS/L2RA.BUN").unwrap();
        let state = Clouds::read(&bytes).unwrap();
        let layer = state.layer.as_ref().expect("authored Clouds layer");
        assert_eq!((layer.node_count(), layer.set_count()), (468, 4));
        for (set, expected) in [(0, (0.5, 0.25)), (1, (0.5, 0.05)), (2, (1.0, 0.05)), (3, (0.2, 0.05))] {
            assert_eq!((layer.float(set, 0), layer.float(set, 1)), (Some(expected.0), Some(expected.1)));
        }
        let mut distinct = std::collections::BTreeSet::new();
        let mut active = 0;
        for x in (-3000..7000).step_by(100) {
            for y in (-5000..4800).step_by(200) {
                let at = [x as f32, y as f32];
                distinct.insert(layer.set_at(at[0], at[1]).unwrap());
                active += usize::from(state.target(at, 0.0) > 0.0);
                assert_eq!(state.target(at, 0.75), 0.75);
            }
        }
        assert!(distinct.len() > 1 && active > 0);
    }
}

#[cfg(test)]
mod comparison_tests {
    use super::*;

    #[test]
    fn manual_clouds_are_immediate_and_location_resume_uses_easing() {
        let mut clouds = Clouds::default();
        clouds.configure(Some(1.0), false, [0.0; 2], 0.0);
        assert_eq!((clouds.intensity(), clouds.target([0.0; 2], 0.0)), (1.0, 1.0));
        clouds.update(0.0, [0.0; 2], 0.0);
        assert_eq!(clouds.intensity(), 1.0);
        clouds.configure(None, false, [0.0; 2], 0.0);
        assert_eq!((clouds.intensity(), clouds.target([0.0; 2], 0.0)), (1.0, 0.0));
        clouds.update(1.0 / 60.0, [0.0; 2], 0.0);
        assert!(clouds.intensity() < 1.0 && clouds.intensity() > 0.99);
        clouds.configure(None, true, [0.0; 2], 0.0);
        assert_eq!(clouds.intensity(), 0.0);
        clouds.configure(Some(0.5), false, [0.0; 2], 1.0);
        assert_eq!(clouds.intensity(), 0.5, "manual comparisons can separate clouds from rain");
        clouds.configure(Some(f32::NAN), true, [0.0; 2], 1.0);
        assert_eq!(clouds.override_amount(), Some(0.5));
        assert_eq!(clouds.intensity(), 0.5);
    }
}
