//! Existing engine-controller RPM feedback for the HUD, independent of the playback backend.

use blackbox_carsound::EngineMixer;

use crate::audio::Audio;

impl Audio {
    /// The selected local voice's visual RPM fraction, once its controller has actually ticked.
    pub fn car_visual_rpm(&self, player: usize) -> Option<f32> {
        let car = match player {
            0 => self.car.as_ref(),
            1 => self.second_car.as_ref(),
            _ => None,
        };
        visual_fraction(car.map(|car| &car.engine))
    }
}

fn visual_fraction(engine: Option<&EngineMixer>) -> Option<f32> {
    let engine = engine?;
    if engine.output().tick_revision == 0 {
        return None;
    }
    let fraction = engine.visual_rpm_fraction();
    (fraction.is_finite() && (0.0..=1.0).contains(&fraction)).then_some(fraction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_carsound::{CarInput, CarSoundTuning, GEAR_FIRST};

    #[test]
    fn missing_and_unticked_voices_have_no_visual_rpm() {
        let engine = EngineMixer::new(&CarSoundTuning::default());
        assert_eq!(visual_fraction(None), None);
        assert_eq!(visual_fraction(Some(&engine)), None);
    }

    #[test]
    fn controller_feedback_preserves_the_pre_race_physics_value() {
        let mut engine = EngineMixer::new(&CarSoundTuning::default());
        let input = CarInput { rpm_pct: 0.625, throttle: 0.5, gear: GEAR_FIRST, pre_race: true, ..Default::default() };
        engine.update(1.0 / 60.0, &input);
        assert_eq!(visual_fraction(Some(&engine)), Some(0.625));
        assert_eq!(visual_fraction(None), None, "stopping a voice must remove its old result");
    }

    #[test]
    #[ignore = "requires NFSMW_GAME_DIR"]
    fn installed_local_visual_rpm_feedback_keeps_voice_identity_and_clears_stopped_voices() {
        use crate::audio::CarSoundState;
        let path = std::env::var_os("NFSMW_GAME_DIR").expect("this check requires NFSMW_GAME_DIR");
        let dir = game_install::GameDir::open(std::path::PathBuf::from(path)).unwrap();
        let mut audio = Audio::replay(dir, 48_000);
        let state = |car: &str, rpm_pct: f32| CarSoundState {
            nitrous_fx: None,
            engine: Default::default(),
            car: car.into(),
            upgrades: Default::default(),
            telemetry: Default::default(),
            input: CarInput { rpm_pct, throttle: 0.5, gear: GEAR_FIRST, pre_race: true, ..Default::default() },
            events: Vec::new(),
            scrape: None,
            surface: 0,
            ticks: Vec::new(),
            view: Default::default(),
        };
        let lead = state("BMWM3GTR", 0.25);
        let guest = state("LANCEREVO8", 0.75);
        audio.drive_car(Some(&lead), 1.0 / 60.0);
        audio.drive_second_car(Some(&guest), 1.0 / 60.0);
        assert_eq!(audio.car_visual_rpm(0), Some(0.25));
        assert_eq!(audio.car_visual_rpm(1), Some(0.75));
        assert_eq!(audio.car_visual_rpm(2), None);
        audio.drive_car(None, 1.0 / 60.0);
        assert_eq!(audio.car_visual_rpm(0), None);
        assert_eq!(audio.car_visual_rpm(1), Some(0.75), "stopping the lead must not clear the guest's voice");
        audio.drive_second_car(None, 1.0 / 60.0);
        assert_eq!(audio.car_visual_rpm(1), None);
    }
}
