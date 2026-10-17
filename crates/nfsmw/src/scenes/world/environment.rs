//! Lighting comparison controls; no simulation step or asset reload on selection.

use super::WorldScene;
use crate::viewer::Environment;

impl WorldScene {
    pub(super) fn environment_state(&self) -> Environment {
        let sun = self.sun.to_sun();
        Environment {
            cloud_override: self.atmosphere.override_amount(),
            cloud_intensity: self.atmosphere.intensity(),
            cloud_target: self.atmosphere.target(self.focus(), self.rain.target()),
            rain_intensity: self.rain.intensity(),
            road_wetness_override: self.wet_road.override_amount,
            road_wetness: self.wet_road.amount(),
            authored_clouds: self.atmosphere.has_authored_map(),
            sun_direction: sun.to_array(),
            dusk_blend: nfsmw_data::time_of_day::dusk_factor(sun.z),
            ..self.day.environment()
        }
    }

    pub(super) fn configure_environment(&mut self, value: Environment) {
        if !value.hour.is_finite()
            || !value.seconds_per_hour.is_finite()
            || !value.sun_phase.is_finite()
            || value.cloud_override.is_some_and(|amount| !amount.is_finite())
            || value.road_wetness_override.is_some_and(|amount| !amount.is_finite())
        {
            return;
        }
        self.configure_day(value);
        self.wet_road.override_amount = value.road_wetness_override.map(|amount| amount.clamp(0.0, 1.0));
        self.atmosphere.configure(value.cloud_override, value.settle_clouds, self.focus(), self.rain.target());
        self.reflection_last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::{Scene, SunMode};

    #[test]
    #[ignore = "needs installed world and weather attributes selected by NFSMW_GAME_DIR"]
    fn installed_comparison_controls_refresh_all_lighting_inputs_without_a_tick() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let mut world = WorldScene::open(
            &dir,
            super::super::Options {
                at: Some([1945.0, 1850.0]),
                height: 2.0,
                heading: 0.0,
                pitch: 0.0,
                fog_distance: 1500.0,
                wait_for_load: false,
                drive: None,
            },
        )
        .unwrap();
        let before_clock = world.clock;
        let mut state = world.environment().unwrap();
        assert!(state.authored_clouds);
        state.sun_mode = SunMode::FixedPhase;
        state.sun_phase = 0.3;
        state.cloud_override = Some(1.0);
        world.set_environment(state);
        let overcast = world.environment().unwrap();
        assert_eq!((overcast.sun_phase, overcast.cloud_intensity, overcast.cloud_target), (0.3, 1.0, 1.0));
        assert_eq!(world.surface_light().unwrap().reflective_specular_scale, 0.0);
        assert!(world.scattering().is_some());
        let old_sun = overcast.sun_direction;
        state.cloud_override = Some(0.0);
        state.sun_phase = 0.5;
        world.set_environment(state);
        let sunny = world.environment().unwrap();
        assert_eq!((sunny.cloud_intensity, sunny.cloud_target), (0.0, 0.0));
        assert_eq!(world.surface_light().unwrap().reflective_specular_scale, 1.0);
        assert_ne!(sunny.sun_direction, old_sun);
        assert!(!sunny.settle_clouds);
        assert_eq!(world.clock, before_clock, "comparison changes must not advance simulation");
        state.cloud_override = Some(f32::NAN);
        world.set_environment(state);
        assert_eq!(world.environment().unwrap(), sunny, "invalid selections hold the last complete state");
    }
}
